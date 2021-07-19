use crate::content_hash::ContentHash;
use crate::interns::{FileId, Interns};
use anyhow::{Context, Result};
use byteorder::LittleEndian;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::Path;
use zerocopy::byteorder::{I64, U32, U64};
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned};

/// File metadata key, based on https://apenwarr.ca/log/20181113
///
/// TODO: Define a different structure for this on Windows.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
#[cfg(any(target_family = "unix", target_endian = "little"))]
struct MetaKey {
    mtime: I64<LittleEndian>,
    size: U64<LittleEndian>,
    ino: U64<LittleEndian>,
    mode: U32<LittleEndian>,
    uid: U32<LittleEndian>,
    gid: U32<LittleEndian>,
}

impl MetaKey {
    pub fn persist(&self, db: &sled::Tree, path: &Path) -> Result<()> {
        db.insert(
            path.to_str().context("this path wasn't UTF-8")?.as_bytes(),
            self.as_bytes(),
        )?;

        Ok(())
    }

    pub fn is_same_as_previous(db: &sled::Tree, path: &Path, current: &Self) -> Result<bool> {
        let entry = db.get(path.to_str().context("this path wasn't UTF-8")?.as_bytes())?;

        match entry {
            Some(previous_bytes) => {
                // ref: https://github.com/spacejam/sled/blob/b23da771902c320bfa20b6f552bebf1d1c1be4ff/examples/structured.rs
                let layout: LayoutVerified<&[u8], MetaKey> =
                    match LayoutVerified::new_unaligned(&*previous_bytes) {
                        Some(layout) => layout,
                        None => panic!("couldn't make a layout from backing bytes"),
                    };

                Ok(current == layout.into_ref())
            }
            None => Ok(false),
        }
    }

    pub fn current(path: &Path) -> Result<Self> {
        // Delegate to an OS-specific internal function.
        Ok(Self::from_metadata(fs::metadata(path)?))
    }

    #[cfg(target_family = "unix")]
    fn from_metadata(metadata: Metadata) -> Self {
        // On UNIX systems, fs::Metadata implements the
        // unix::fs::MetadataExt trait, giving us access
        // to UNIX-specific file metadata like uid and gid.
        use std::os::unix::fs::MetadataExt;

        Self {
            mtime: metadata.mtime().into(),
            size: metadata.size().into(),
            ino: metadata.ino().into(),
            mode: metadata.mode().into(),
            uid: metadata.uid().into(),
            gid: metadata.gid().into(),
        }
    }
}

pub struct Cache {
    by_file_id: HashMap<FileId, (MetaKey, ContentHash)>,
    metakeys: sled::Tree,
    hashes: sled::Tree,
}

impl Cache {
    pub fn new(db_path: &Path) -> Result<Self> {
        let db = sled::Config::default()
            .path(db_path)
            .mode(sled::Mode::HighThroughput)
            .open()?;
        Ok(Cache {
            by_file_id: HashMap::default(),
            metakeys: db
                .open_tree("metakeys")
                .context("couldn't open metakeys tree")?,
            hashes: db
                .open_tree("hashes")
                .context("couldn't open hashes tree")?,
        })
    }

    /// Iterate through each of the given FileId entries and call
    /// self.content_changed on them, then return a map of all the files
    /// that changed.
    pub fn find_changed<'a, I: Iterator<Item = &'a FileId>>(
        &mut self,
        file_ids: I,
        interns: &Interns,
    ) -> Result<HashMap<FileId, ContentHash>> {
        let mut changed = HashMap::default();

        // If any changed, add them to the map.
        for &file_id in file_ids {
            if let Some(hash) = self.content_changed(file_id, interns)? {
                changed.insert(file_id, hash);
            }
        }

        Ok(changed)
    }

    /// Check whether the content of the file ID changed.
    /// This is done by reading it from disk and seeing if the file's
    /// metadata changed since the last time we looked at it. If the metadata
    /// is the same, we assume the contents have not changed, and we
    /// return a cached ContentHash. If the metadata is different, we
    /// read the contents of the file from disk and hash them into a ContentHash,
    /// then cache that ContentHash under the metadata for future use.
    ///
    /// If the ContentHash hasn't changed, return None.
    ///
    /// This operation is not atomic! If the file changes
    /// (according to `notify`) in the middle of this operation, this will
    /// need to be re-run on that file.
    pub fn content_changed(
        &mut self,
        file_id: FileId,
        interns: &Interns,
    ) -> Result<Option<ContentHash>> {
        // We should definitely have an Interns entry for this file_id!
        let path = interns.get_path(file_id).unwrap_or_else(|| unreachable!());

        // If the file's current metadata is the same as the last one we
        // recorded on disk, then we can reasonably conclude it hasn't changed.
        let current_meta_key = MetaKey::current(path)?;

        if MetaKey::is_same_as_previous(&self.metakeys, path, &current_meta_key)? {
            Ok(None)
        } else {
            // The metadata was different, so the file may have changed.
            // Proceed with computing the ContentHash from the file's contents!

            // Read the file from the file system and hash it
            let current_hash = ContentHash::from_file(path)?;
            let prev_hash;

            // To find the previous ContentHash for this FileId, try the in-memory
            // ContentHash cache first before going to the on-disk cache.
            match self.by_file_id.get(&file_id) {
                Some((_stale_meta_key, hash)) => {
                    prev_hash = Some(*hash);

                    // Record the new MetaKey in the in-memory cache.
                    self.by_file_id
                        .insert(file_id, (current_meta_key, current_hash));
                }
                None => {
                    // We don't have this one in memory, so
                    // try the on-disk cache.
                    match self.get_cached_hash(path)? {
                        Some(hash) => {
                            // Save the on-disk hash in our in-memory cache, so
                            // we don't have to read it from disk again next time.
                            self.by_file_id.insert(file_id, (current_meta_key, hash));

                            prev_hash = Some(hash);
                        }
                        None => {
                            // We've never hashed this file before.
                            //
                            // Store it in both the in-memory cache
                            // as well as on disk for future runs.
                            self.by_file_id
                                .insert(file_id, (current_meta_key, current_hash));
                            self.persist(path, current_hash)?;

                            // We've never seen this content before. This will
                            // have the effect that we end up considering it
                            // changed!
                            prev_hash = None;
                        }
                    }
                }
            };

            // Now that we've made it past the point where we might have returned
            // early with an io::Err, we should record the new MetaKey. This way,
            // the next time we ask whether this path has changed, we'll be
            // considering it relative to the ContentHash we're about to return.
            current_meta_key.persist(&self.metakeys, path)?;

            if Some(current_hash) == prev_hash {
                // The file's content has not changed.
                Ok(None)
            } else {
                // The file's content hash has changed!
                Ok(Some(current_hash))
            }
        }
    }

    fn get_cached_hash(&self, path: &Path) -> Result<Option<ContentHash>> {
        // first, look up the given path in the
        // (Path => (FileMetadata, ContentHash)) cache. If we have an entry,
        // then compare the file metadata to that file's current metadata; if
        // it's unchanged, then we can use the given ContentHash.
        // If that has an entry, then we have our
        self.hashes
            .get(path.to_str().context("this path wasn't UTF-8")?.as_bytes())
            .map(|entry| {
                entry.map(|previous_bytes| {
                    // ref: https://github.com/spacejam/sled/blob/b23da771902c320bfa20b6f552bebf1d1c1be4ff/examples/structured.rs
                    let layout: LayoutVerified<&[u8], ContentHash> =
                        match LayoutVerified::new_unaligned(&*previous_bytes) {
                            Some(layout) => layout,
                            None => panic!("couldn't make a layout from backing bytes"),
                        };

                    *layout.into_ref()
                })
            })
            .context("couldn't retrieve the hash from disk")
    }

    fn persist(&self, path: &Path, hash: ContentHash) -> Result<()> {
        // TODO convert the path to be relative to the cache dir itself,
        // so you don't lose everything if you rename the project directory -
        // and also on a build server you can copy it to different builds in
        // different directories, so they can have a cache to start out with.
        //
        // TODO: how can we make renames efficient without invalidating the old
        // hashes? e.g. so if we switch branches, we don't have to rebuild everything?
        self.hashes.insert(
            path.to_str().context("this path wasn't UTF-8")?.as_bytes(),
            hash.as_bytes(),
        )?;

        Ok(())
    }
}
