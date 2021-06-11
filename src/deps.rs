use crate::cache::Cache;
use crate::content_hash::ContentHash;
use crate::interns::{FileId, Interns};
use std::collections::{HashMap, HashSet};
use std::io;
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
    pub fn find_changed(&mut self, cache: &mut Cache) -> io::Result<HashMap<FileId, ContentHash>> {
        cache.find_changed(self.all.iter(), &self.interns)
    }

    /// Given a root, recursively add everything that depends on it.
    pub fn add_deps<F: Fn(&Path) -> &'a [&'a Path]>(&mut self, root: &'a Path, get_deps: &F) {
        let deps = get_deps(root);

        self.register_deps(root, deps);

        for dep in deps {
            self.add_deps(dep, get_deps);
        }
    }

    fn register_deps(&mut self, root: &'a Path, depends_on: &[&'a Path]) {
        let interns = &mut self.interns;
        let root_id = interns.get_or_add(root);
        let deps_set = self.by_root.entry(root_id).or_default();

        for dep in depends_on {
            let dep_id = interns.get_or_add(dep);

            deps_set.insert(dep_id);

            let roots_set = self.by_dep.entry(dep_id).or_default();

            roots_set.insert(root_id);
        }
    }
}
