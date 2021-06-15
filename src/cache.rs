use crate::content_hash::ContentHash;
use crate::interns::{FileId, Interns};
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::io;
use std::path::Path;

/// File metadata key, based on https://apenwarr.ca/log/20181113
///
/// TODO: Define a different structure for this on Windows.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
#[cfg(target_family = "unix")]
struct MetaKey {
    mtime: i64,
    size: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
}

impl MetaKey {
    pub fn persist(&self, _path: &Path) -> io::Result<()> {
        // TODO store a (Path => MetaKey) dictionary entry on disk.
        // For now, don't persist at all.
        Ok(())
    }

    pub fn stored(_path: &Path) -> io::Result<Option<Self>> {
        // TODO get the stored MetaKey from the (Path => MetaKey) dictionary on disk
        // For now, assume nothing was stored.
        Ok(None)
    }

    pub fn current(path: &Path) -> io::Result<Self> {
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
            mtime: metadata.mtime(),
            size: metadata.size(),
            ino: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        }
    }
}

#[derive(Default)]
pub struct Cache {
    by_file_id: HashMap<FileId, (MetaKey, ContentHash)>,
}

impl Cache {
    /// Iterate through each of the given FileId entries and call
    /// self.content_changed on them, then return a map of all the files
    /// that changed.
    pub fn find_changed<'a, I: Iterator<Item = &'a FileId>>(
        &mut self,
        file_ids: I,
        interns: &Interns,
    ) -> io::Result<HashMap<FileId, ContentHash>> {
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
    ) -> io::Result<Option<ContentHash>> {
        // We should definitely have an Interns entry for this file_id!
        let path = interns.get_path(file_id).unwrap_or_else(|| unreachable!());

        // If the file's current metadata is the same as the last one we
        // recorded on disk, then we can reasonably conclude it hasn't changed.
        let prev_meta_key = MetaKey::stored(path)?;
        let current_meta_key = MetaKey::current(path)?;

        if Some(current_meta_key) == prev_meta_key {
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
                    match Self::lookup_on_disk(path)? {
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
                            Self::persist(path, current_hash)?;

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
            current_meta_key.persist(path)?;

            if Some(current_hash) == prev_hash {
                // The file's content has not changed.
                Ok(None)
            } else {
                // The file's content hash has changed!
                Ok(Some(current_hash))
            }
        }
    }

    fn lookup_on_disk(_path: &Path) -> io::Result<Option<ContentHash>> {
        // first, look up the given path in the
        // (Path => (FileMetadata, ContentHash)) cache. If we have an entry,
        // then compare the file metadata to that file's current metadata; if
        // it's unchanged, then we can use the given ContentHash.
        // If that has an entry, then we have our
        Ok(None)
    }

    fn persist(_path: &Path, _hash: ContentHash) -> io::Result<()> {
        // TODO convert the path to be relative to the cache dir itself,
        // so you don't lose everything if you rename the project directory -
        // and also on a build server you can copy it to different builds in
        // different directories, so they can have a cache to start out with.
        //
        // TODO: how can we make renames efficient without invalidating the old
        // hashes? e.g. so if we switch branches, we don't have to rebuild everything?
        Ok(())
    }
}
