use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub inputs: Vec<PathBuf>,
}

impl Job {
    pub fn run(&self) -> io::Result<Output> {
        Command::new(self.command.as_str())
            .args(
                self.arguments
                    .iter()
                    .map(|arg| arg.as_str())
                    .collect::<Vec<&str>>()
                    .as_slice(),
            )
            // TODO: this is going to have to retain some environment variables
            // for software to work correctly. For example, we'll probably need
            // to provide a fake HOME the way Nix does.
            .env_clear()
            .output()
    }
}

#[cfg(test)]
mod test_job {
    use super::Job;
    use std::collections::HashMap;
    use std::env;
    use std::fs::File;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn produces_output() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("hello.txt");

        let job = Job {
            command: "bash".to_string(),
            arguments: vec![
                "-c".to_string(),
                format!("echo Hello, World > {}", dest.to_str().unwrap()),
            ],
            environment: HashMap::default(),
            inputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(output.status.success(), true);

        let contents = std::fs::read_to_string(dest).unwrap();
        assert_eq!(contents.as_str(), "Hello, World\n");
    }

    #[test]
    fn captures_stdout() {
        let job = Job {
            command: "echo".to_string(),
            arguments: vec!["Hello, Stdout!".to_string()],
            environment: HashMap::default(),
            inputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "Hello, Stdout!\n".to_string()
        );
    }

    #[test]
    fn captures_stderr() {
        let job = Job {
            command: "bash".to_string(),
            arguments: vec!["-c".to_string(), "echo 'Hello, Stderr!' 1>&2".to_string()],
            environment: HashMap::default(),
            inputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Hello, Stderr!\n".to_string()
        );
    }

    #[test]
    fn reports_a_problem() {
        let job = Job {
            command: "bash".to_string(),
            arguments: vec!["-c".to_string(), "exit 1".to_string()],
            environment: HashMap::default(),
            inputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(output.status.success(), false);
    }

    #[test]
    fn isolates_environment() {
        let job = Job {
            command: "env".to_string(),
            arguments: vec![],
            environment: HashMap::default(),
            inputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(String::from_utf8(output.stderr).unwrap(), "".to_string());
    }

    #[test]
    fn only_inputs_are_visible() {
        let temp = EnterTempDir::start();

        let visible = PathBuf::from("visible.txt");
        File::create(&visible).unwrap();

        let hidden = PathBuf::from("hidden.txt");
        File::create(hidden).unwrap();

        let job = Job {
            command: "find".to_string(),
            arguments: vec![".".to_string()],
            environment: HashMap::default(),
            inputs: vec![visible],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            ".\n./visible.txt\n".to_string()
        );

        drop(temp);
    }

    // helper: move into a temporary directory for the duration of the test
    struct EnterTempDir {
        original: PathBuf,
        new: tempfile::TempDir,
    }

    impl EnterTempDir {
        fn start() -> EnterTempDir {
            let original = env::current_dir().unwrap();
            let new = tempdir().unwrap();

            env::set_current_dir(new.path()).unwrap();

            EnterTempDir { original, new }
        }
    }

    impl Drop for EnterTempDir {
        fn drop(&mut self) {
            env::set_current_dir(&self.original).unwrap();
            drop(&self.new)
        }
    }
}
