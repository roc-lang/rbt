use std::collections::HashMap;
use std::hash::Hash;

/// A number which can be given to an Interns table to obtain a Path.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileId(usize);

trait Id {
    const FIRST_NON_RESERVED: Self;

    fn inc(self) -> Option<Self>;
}

impl Id for FileId {
    const FIRST_NON_RESERVED: Self = Self(1);

    fn inc(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(incremented) => Some(Self(incremented)),
            None => None,
        }
    }
}

impl FileId {
    /// FileId 0 is reserved for NULL
    pub const _NULL: FileId = FileId(0);
}

/// A table mapping Path values to FileId values. This allows for using
/// FileId integers in things like repeated equality comparisons and hashing
/// operations, instead of having to hash or compare lots of full Path strings.
#[derive(Debug)]
pub struct Interns<'a, Id: Hash + Eq + self::Id, V: Hash + Eq> {
    by_val: HashMap<V, Id>,
    by_id: HashMap<Id, V>,
    next_id: Id,
}

impl<'a, V: Hash + Eq, Id: Hash + Eq + self::Id> Default for Interns<'a, Id, V> {
    fn default() -> Self {
        Self {
            by_val: HashMap::default(),
            by_id: HashMap::default(),
            next_id: Id::FIRST_NON_RESERVED,
        }
    }
}

impl<'a, V: Hash + Eq, Id: Hash + Eq + self::Id> Interns<'a, Id, V> {
    // clippy thinks this is unused, even though it is used in Deps. Go figure.
    #[allow(dead_code)]
    pub fn get_id(&self, val: &V) -> Option<&Id> {
        self.by_val.get(val)
    }

    pub fn get_val(&self, id: &Id) -> Option<&V> {
        self.by_id.get(id)
    }

    pub fn get_or_add(&mut self, val: V) -> Id {
        use std::collections::hash_map::Entry::*;

        match self.by_val.entry(val) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => match self.next_id.inc() {
                Some(incremented_id) => {
                    let id = self.next_id;

                    entry.insert(id);

                    self.by_id.insert(id, val);
                    self.next_id = incremented_id;

                    id
                }
                None => {
                    // Our usize overflowed!
                    panic!("Ran out of FileIds!");
                }
            },
        }
    }
}
