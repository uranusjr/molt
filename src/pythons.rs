use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempdir::TempDir;
use which;

use crate::vendors::Vendor;

#[derive(Debug)]
pub enum Error {
    LookupError(which::Error),
    InvocationError(io::Error),
    UnrepresentableError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::LookupError(ref e) => e.fmt(f),
            Error::InvocationError(ref e) => e.fmt(f),
            Error::UnrepresentableError => {
                write!(f, "Interpreter path not representable")
            },
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::InvocationError(e)
    }
}

impl From<which::Error> for Error {
    fn from(e: which::Error) -> Error {
        Error::LookupError(e)
    }
}

type Result<T> = std::result::Result<T, Error>;


fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd
}


#[allow(dead_code)]
pub struct Interpreter {
    location: PathBuf,
}

impl Interpreter {
    pub fn discover<I, S>(program: S, args: I) -> Result<Self>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        let out = command(&which::which(program)?)
            .args(args)
            .args(&[
                "-c",
                "from __future__ import print_function; \
                 import sys; print(sys.executable, end='')",
            ])
            .output()?;

        // This cannot fail because we told the interpreter to use UTF-8.
        let location = PathBuf::from(String::from_utf8(out.stdout).unwrap());
        Ok(Self { location: location })
    }

    pub fn create_venv(&self, env_dir: &Path, prompt: &str) -> Result<()> {
        let tmp_dir = TempDir::new("molt-venv")?;
        Vendor::populate_to(tmp_dir.path())?;

        let code = format!(
            "import virtenv; virtenv.create(None, {:?}, False, prompt={:?})",
            env_dir.to_string_lossy().into_owned(),
            prompt,
        );

        // TODO: Show message based on status code.
        let _status = command(&self.location)
            .env("PYTHONPATH",
                 tmp_dir.path().to_str().ok_or(Error::UnrepresentableError)?)
            .args(&["-c", &code])
            .spawn()?
            .wait()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_discover() {
        let result = Interpreter::discover(&"python", &[]);
        assert!(result.is_ok());
    }
}
