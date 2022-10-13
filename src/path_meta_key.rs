use anyhow::{Context, Result};
use std::convert::TryFrom;
use std::fs::Metadata;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;
use xxhash_rust::xxh3::Xxh3;

#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Hash)]
pub struct PathMetaKey {
    // common
    modified: SystemTime,
    len: u64,

    // Unix-only
    #[cfg(target_family = "unix")]
    inode: u64,
    #[cfg(target_family = "unix")]
    mode: u32,
    #[cfg(target_family = "unix")]
    uid: u32,
    #[cfg(target_family = "unix")]
    gid: u32,
    // TODO: extra info for Windows
}

impl PathMetaKey {
    pub fn to_db_key(&self) -> [u8; 8] {
        let mut hasher = Xxh3::new();
        self.hash(&mut hasher);

        hasher.finish().to_le_bytes()
    }
}

#[cfg(target_family = "unix")]
impl TryFrom<Metadata> for PathMetaKey {
    type Error = anyhow::Error;

    fn try_from(meta: Metadata) -> Result<PathMetaKey> {
        Ok(PathMetaKey {
            modified: meta
                .modified()
                .context("mtime is not supported on this system")?,
            len: meta.len(),
            inode: meta.ino(),
            mode: meta.mode(),
            uid: meta.uid(),
            gid: meta.gid(),
        })
    }
}

#[cfg(not(target_family = "unix"))]
impl TryFrom<Metadata> for PathMetaKey {
    type Error = anyhow::Error;

    fn try_from(meta: Metadata) -> Result<PathMetaKey> {
        Ok(PathMetaKey {
            modified: meta
                .modified()
                .context("mtime is not supported on this system")?,
            len: meta.len(),
        })
    }
}
