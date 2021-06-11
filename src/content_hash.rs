use digest::Digest;
use meowhash::MeowHasher;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct ContentHash {
    /// A MeowHash digest <https://mollyrocket.com/meowhash>
    ///
    /// This 128-bit hash is designed to never have collisions in practice, and
    /// to run super fast on files of substantial size. Exactly what we want!
    bits: u128,
}

impl ContentHash {
    /// Read the contents of a file and translate them into a ContentHash
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut hasher = MeowHasher::new();
        let mut buffer = [0; 1024];

        loop {
            let bytes_read = reader.read(&mut buffer)?;

            if bytes_read != 0 {
                hasher.update(&buffer[..bytes_read]);
            } else {
                break;
            }
        }

        let bits = hasher.finalise().as_u128();

        Ok(Self { bits })
    }
}
