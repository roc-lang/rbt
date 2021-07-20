use crate::cache::Cache;
use crate::content_hash::ContentHash;
use crate::interns::{FileId, Interns};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Default, Debug)]
pub struct Deps<'a> {
    /// For each root, what are its dependencies?
    by_root: HashMap<FileId, HashSet<FileId>>,

    /// For each dependency, which roots depend on it?
    by_dep: HashMap<FileId, HashSet<FileId>>,

    /// All the roots and all their deps
    all: HashSet<FileId>,

    interns: Interns<'a>,
}

impl<'a> Deps<'a> {
    /// Among all the known roots - and their dependencies - find all the
    /// individual files that have changes on disk (compared to the cache).
    pub fn find_changed(&mut self, cache: &mut Cache) -> Result<HashMap<FileId, ContentHash>> {
        cache.find_changed(self.all.iter(), &self.interns)
    }

    /// Given a root, recursively add everything that depends on it.
    pub fn add<F: Fn(&Path) -> &'a [&'a Path]>(&mut self, root: &'a Path, get_deps: &F) {
        let deps = get_deps(root);

        self.register(root, deps);

        for dep in deps {
            self.add(dep, get_deps);
        }
    }

    fn register(&mut self, root: &'a Path, depends_on: &[&'a Path]) {
        let interns = &mut self.interns;
        let root_id = interns.get_or_add(root);
        let deps_set = self.by_root.entry(root_id).or_default();
        let all = &mut self.all;

        all.insert(root_id);

        for dep in depends_on {
            let dep_id = interns.get_or_add(dep);

            all.insert(dep_id);

            deps_set.insert(dep_id);

            let roots_set = self.by_dep.entry(dep_id).or_default();

            roots_set.insert(root_id);
        }
    }
}

#[cfg(test)]
mod test_deps {
    use super::Deps;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn add_secondary_deps() {
        let mut deps = Deps::default();
        let root = Path::new("tests/fixtures/entry.txt");
        let secondary_deps = &[
            Path::new("tests/fixtures/alice.txt"),
            Path::new("tests/fixtures/small.txt"),
        ];
        let secondary_deps_set = {
            let mut set: HashSet<&Path> = HashSet::default();

            for path in secondary_deps.iter() {
                set.insert(path);
            }

            set
        };

        deps.add(root, &|path| {
            if path == root {
                secondary_deps
            } else {
                &[]
            }
        });

        assert_eq!(deps.by_root.len(), 3);
        assert_eq!(deps.by_dep.len(), 2);
        assert_eq!(deps.all.len(), 3);

        // The original root should have the expected 2 dependencies
        {
            let root_id = deps.interns.get_id(root).unwrap();
            let mut set = HashSet::default();

            for id in deps.by_root.get(&root_id).unwrap() {
                set.insert(deps.interns.get_path(*id).unwrap());
            }

            assert_eq!(set, secondary_deps_set);
        }

        // The root's dependencies should have no other dependencies
        {
            let original_root_id = deps.interns.get_id(root).unwrap();
            for root_id in deps.all {
                if root_id != original_root_id {
                    assert_eq!(0, deps.by_root.get(&root_id).unwrap().len());

                    let id_set = deps.by_dep.get(&root_id).unwrap();

                    assert_eq!(id_set.len(), 1);
                    assert!(id_set.contains(&original_root_id));
                }
            }
        }
    }
}
