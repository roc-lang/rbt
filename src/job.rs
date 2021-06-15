use std::collections::HashMap;
use std::io;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
}

impl Job {
    pub fn run(&self) -> io::Result<()> {
        match Command::new(self.command.as_str())
            .args(
                self.arguments
                    .iter()
                    .map(|arg| arg.as_str())
                    .collect::<Vec<&str>>()
                    .as_slice(),
            )
            .status()
        {
            Ok(_) => Ok(()),
            Err(_) => todo!(),
        }
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

        assert_eq!(job.run().unwrap(), ());

        let contents = std::fs::read_to_string(dest).unwrap();
        assert_eq!(contents.as_str(), "Hello, World\n");
    }

    // fn reports_a_problem() {
    // }
}
