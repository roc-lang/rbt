use std::collections::HashMap;
use std::io;
use std::process::{Command, Output};

#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
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
            .output()
    }
}

#[cfg(test)]
mod test_job {
    use super::Job;
    use std::collections::HashMap;
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
        };

        let output = job.run().unwrap();
        assert_eq!(output.status.success(), false);
    }
}
