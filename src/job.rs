use crate::glue;
use std::hash::{Hash, Hasher};

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct Id(u64);

impl From<u64> for Id {
    fn from(unwrapped: u64) -> Self {
        Id(unwrapped)
    }
}

impl From<&glue::Job> for Id {
    fn from(job: &glue::Job) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        job.hash(&mut hasher);

        Id(hasher.finish())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}
