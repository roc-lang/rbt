use std::collections::HashMap;
use std::path::Path;

/// A number which can be given to an Interns table to obtain a Path.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileId(usize);

impl FileId {
    /// FileId 0 is reserved for NULL
    pub const _NULL: FileId = FileId(0);

    const FIRST_NON_RESERVED_ID: FileId = FileId(1);
}

/// A table mapping Path values to FileId values. This allows for using
/// FileId integers in things like repeated equality comparisons and hashing
/// operations, instead of having to hash or compare lots of full Path strings.
#[derive(Debug)]
pub struct Interns<'a> {
    by_path: HashMap<&'a Path, FileId>,
    by_id: HashMap<FileId, &'a Path>,
    next_id: FileId,
}

impl<'a> Default for Interns<'a> {
    fn default() -> Self {
        Self {
            by_path: HashMap::default(),
            by_id: HashMap::default(),
            next_id: FileId::FIRST_NON_RESERVED_ID,
        }
    }
}

impl<'a> Interns<'a> {
    // clippy thinks this is unused, even though it is used in Deps. Go figure.
    #[allow(dead_code)]
    pub fn get_id(&self, path: &'a Path) -> Option<FileId> {
        self.by_path.get(path).copied()
    }

    pub fn get_path(&self, file_id: FileId) -> Option<&'a Path> {
        self.by_id.get(&file_id).copied()
    }

    pub fn get_or_add(&mut self, path: &'a Path) -> FileId {
        use std::collections::hash_map::Entry::*;

        match self.by_path.entry(path) {
            Occupied(entry) => *entry.get(),
            Vacant(entry) => match self.next_id.0.checked_add(1) {
                Some(next_id_raw) => {
                    let id = self.next_id;

                    entry.insert(id);

                    self.by_id.insert(id, path);
                    self.next_id = FileId(next_id_raw);

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

#[cfg(test)]
mod test_interns {
    use super::Interns;
    use std::path::Path;

    #[test]
    fn multiple_get_or_add() {
        let path1 = Path::new(".");
        let path2 = Path::new("./blah");

        let mut interns = Interns::default();
        let id1 = interns.get_or_add(&path1);
        let id2 = interns.get_or_add(&path2);

        assert_eq!(Some(path1), interns.get_path(id1));
        assert_eq!(Some(path2), interns.get_path(id2));
        assert_ne!(id1, id2);
        assert_ne!(path1, path2);
    }
}
