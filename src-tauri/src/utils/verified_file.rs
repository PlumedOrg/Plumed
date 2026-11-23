//! A file written chunk by chunk and accepted only if its size and SHA-256
//! are the expected ones (Plumed fork, written with Claude Code). Used by the
//! medical data updates and the speech model downloads; the latter resume an
//! interrupted download by reopening its partial file.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug)]
pub enum VerifyError {
    /// More bytes than expected, or the wrong size or hash at the end.
    Mismatch,
    Io(io::Error),
}

impl From<io::Error> for VerifyError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

pub struct VerifiedFile {
    out: fs::File,
    hash: ring::digest::Context,
    written: u64,
    size: u64,
    sha256: String,
}

impl VerifiedFile {
    /// Creates (or empties) `path`. `sha256` is lowercase hex.
    pub fn create(path: &Path, size: u64, sha256: &str) -> io::Result<Self> {
        Ok(Self::new(fs::File::create(path)?, size, sha256))
    }

    /// Reopens `path` to append to what it holds, hashing that first. Starts
    /// over when it holds more than `size` bytes.
    pub fn resume(path: &Path, size: u64, sha256: &str) -> io::Result<Self> {
        let held = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if held == 0 || held > size {
            return Self::create(path, size, sha256);
        }
        let mut file = Self::new(
            fs::OpenOptions::new().append(true).open(path)?,
            size,
            sha256,
        );
        file.written = hash_into(&mut file.hash, fs::File::open(path)?)?;
        Ok(file)
    }

    fn new(out: fs::File, size: u64, sha256: &str) -> Self {
        Self {
            out,
            hash: ring::digest::Context::new(&ring::digest::SHA256),
            written: 0,
            size,
            sha256: sha256.to_owned(),
        }
    }

    pub fn written(&self) -> u64 {
        self.written
    }

    pub fn write(&mut self, chunk: &[u8]) -> Result<(), VerifyError> {
        self.written += chunk.len() as u64;
        if self.written > self.size {
            return Err(VerifyError::Mismatch);
        }
        self.out.write_all(chunk)?;
        self.hash.update(chunk);
        Ok(())
    }

    /// Checks the size and hash, and flushes the file to disk.
    pub fn finish(self) -> Result<(), VerifyError> {
        if self.written != self.size || hex_of(self.hash.finish().as_ref()) != self.sha256 {
            return Err(VerifyError::Mismatch);
        }
        self.out.sync_all()?;
        Ok(())
    }
}

/// Lowercase hex SHA-256 of the file at `path`, read in chunks.
pub fn file_sha256(path: &Path) -> io::Result<String> {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash_into(&mut hash, fs::File::open(path)?)?;
    Ok(hex_of(hash.finish().as_ref()))
}

pub fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_into(hash: &mut ring::digest::Context, mut reader: impl Read) -> io::Result<u64> {
    let mut buffer = vec![0u8; 1 << 20];
    let mut total = 0u64;
    loop {
        match reader.read(&mut buffer)? {
            0 => return Ok(total),
            n => {
                hash.update(&buffer[..n]);
                total += n as u64;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("plumed-verified-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn accepts_the_expected_bytes_and_refuses_others() {
        let path = temp("a");
        let mut file = VerifiedFile::create(&path, 5, HELLO_SHA).unwrap();
        file.write(b"hel").unwrap();
        file.write(b"lo").unwrap();
        assert!(file.finish().is_ok());
        assert_eq!(file_sha256(&path).unwrap(), HELLO_SHA);

        let mut wrong = VerifiedFile::create(&path, 5, HELLO_SHA).unwrap();
        wrong.write(b"jello").unwrap();
        assert!(matches!(wrong.finish(), Err(VerifyError::Mismatch)));

        let mut long = VerifiedFile::create(&path, 5, HELLO_SHA).unwrap();
        assert!(matches!(long.write(b"hello!"), Err(VerifyError::Mismatch)));

        let mut short = VerifiedFile::create(&path, 5, HELLO_SHA).unwrap();
        short.write(b"hell").unwrap();
        assert!(matches!(short.finish(), Err(VerifyError::Mismatch)));
    }

    #[test]
    fn resumes_a_partial_file_and_restarts_an_oversized_one() {
        let path = temp("b");
        fs::write(&path, b"hel").unwrap();
        let mut file = VerifiedFile::resume(&path, 5, HELLO_SHA).unwrap();
        assert_eq!(file.written(), 3);
        file.write(b"lo").unwrap();
        assert!(file.finish().is_ok());
        assert_eq!(fs::read(&path).unwrap(), b"hello");

        fs::write(&path, b"hello world").unwrap();
        let file = VerifiedFile::resume(&path, 5, HELLO_SHA).unwrap();
        assert_eq!(file.written(), 0);
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);

        let missing = path.with_file_name("missing");
        assert_eq!(
            VerifiedFile::resume(&missing, 5, HELLO_SHA)
                .unwrap()
                .written(),
            0
        );
    }
}
