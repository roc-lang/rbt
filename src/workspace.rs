use crate::{job, store};
use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_family = "unix")]
use std::os::unix::fs::symlink;

#[cfg(target_family = "windows")]
use std::os::windows::fs::symlink_file;

#[derive(Debug)]
pub struct Workspace(PathBuf);

impl Workspace {
    pub fn create<Finality>(root: &Path, key: &job::Key<Finality>) -> Result<Self> {
        let workspace = Workspace(root.join("workspaces").join(key.to_string()));

        std::fs::create_dir_all(&workspace.0).context("could not create workspace")?;

        Ok(workspace)
    }

    pub fn set_up_files(
        &self,
        job: &job::Job,
        job_to_store_path: &HashMap<job::Key<job::Base>, store::Item>,
    ) -> Result<()> {
        for file in &job.input_files {
            self.set_up_path(file, file)?
        }

        for (key, files) in &job.input_jobs {
            let store_item = job_to_store_path
                .get(key)
                .with_context(|| format!("could not find a store path for job {}", key))?;

            for file in files {
                self.set_up_path(&store_item.join(file), file)?
            }
        }

        Ok(())
    }

    fn set_up_path(&self, src: &Path, dest: &Path) -> Result<()> {
        // validate that the path exists and is a file
        let meta = src
            .metadata()
            .with_context(|| format!("`{}` does not exist", dest.display()))?;

        if meta.is_dir() {
            anyhow::bail!(
                "`{}` was a directory, but workspace source paths can only be files",
                src.display()
            )
        }

        if let Some(parent_base) = dest.parent() {
            let parent = self.join(parent_base);

            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("could not create parent for `{}`", dest.display()))?;
            }
        }

        let absolute_src = src.absolutize().with_context(|| {
            format!("could not convert `{}` to an absolute path", src.display())
        })?;

        #[cfg(target_family = "unix")]
        symlink(absolute_src, self.join(dest))
            .with_context(|| format!("could not symlink `{}` into workspace", dest.display()))?;

        #[cfg(target_family = "windows")]
        symlink_file(absolute_src, workspace.join(dest))
            .with_context(|| format!("could not symlink `{}` into workspace", file.display()))?;

        Ok(())
    }

    pub fn join<P: AsRef<Path>>(&self, other: P) -> PathBuf {
        self.0.join(other)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let Err(problem) = std::fs::remove_dir_all(&self.0) {
            log::warn!("problem removing workspace dir: {}", problem);
        };
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glue;
    use roc_std::{RocDict, RocList, RocStr};
    use std::{collections::HashMap, path::PathBuf};
    use tempfile::TempDir;

    fn key() -> job::Key<job::Final> {
        job::Key::default()
    }

    fn glue_job_with_files<'roc>(files: &[&str]) -> glue::Job {
        glue::Job::Job(glue::R1 {
            command: glue::Command {
                tool: glue::Tool::SystemTool(glue::SystemToolPayload {
                    name: RocStr::from("bash"),
                }),
                args: RocList::empty(),
            },
            inputs: RocList::from_slice(&[glue::U1::FromProjectSource(
                files
                    .iter()
                    .map(|name| (*name).into())
                    .collect::<RocList<RocStr>>(),
            )]),
            outputs: RocList::empty(),
            env: RocDict::with_capacity(0),
        })
    }

    #[test]
    fn sets_up_and_tears_down() {
        let temp = TempDir::new().unwrap();

        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");
        let path = workspace.as_ref().to_path_buf();

        assert!(path.is_dir());

        drop(workspace);

        assert!(!path.exists());
    }

    #[test]
    fn test_sets_up_file() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");

        let glue_job = glue_job_with_files(&[file!()]);
        let job = job::Job::from_glue(&glue_job, &HashMap::new()).unwrap();
        workspace
            .set_up_files(&job, &HashMap::new())
            .expect("failed to set up files");

        let path = workspace.join(file!());

        assert!(path.is_symlink());
        assert_eq!(
            PathBuf::from(file!()).absolutize().unwrap(),
            path.read_link().unwrap()
        );
    }

    #[test]
    fn test_rejects_missing_file() {
        let temp = TempDir::new().unwrap();

        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");
        let glue_job = glue_job_with_files(&["does-not-exist"]);
        let job = job::Job::from_glue(&glue_job, &HashMap::new()).unwrap();

        assert_eq!(
            String::from("`does-not-exist` does not exist"),
            workspace
                .set_up_files(&job, &HashMap::new())
                .unwrap_err()
                .to_string(),
        )
    }

    #[test]
    fn test_rejects_directory() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");

        // currently, `file!()` gives us `src/workspace.rs`. This works for us at
        // the moment, but all we really need is a path containing a directory.
        let here = PathBuf::from(file!());
        let parent = here.parent().unwrap();

        let glue_job = glue_job_with_files(&[parent.to_str().unwrap()]);
        let job = job::Job::from_glue(&glue_job, &HashMap::new()).unwrap();

        assert_eq!(
            format!(
                "`{}` was a directory, but workspace source paths can only be files",
                parent.display()
            ),
            workspace
                .set_up_files(&job, &HashMap::new())
                .unwrap_err()
                .to_string()
        );
    }
}
