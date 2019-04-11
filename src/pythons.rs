use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use which;

#[derive(Debug)]
pub enum Error {
    LookupError(which::Error),
    InvocationError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::LookupError(ref e) => e.fmt(f),
            Error::InvocationError(ref e) => e.fmt(f),
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


#[allow(dead_code)]
pub struct Interpreter {
    location: PathBuf,
}

impl Interpreter {
    pub fn discover<I, S>(program: S, args: I) -> Result<Self>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        let out = Command::new(which::which(program)?)
            .env("PYTHONIOENCODING", "utf-8")
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
