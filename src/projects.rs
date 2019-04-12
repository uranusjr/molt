use std::env::current_dir;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use unindent::unindent;

use crate::entrypoints::EntryPoints;
use crate::pythons::{self, Interpreter};

#[derive(Debug)]
pub enum Error {
    CommandNotFoundError(String),
    EnvironmentNotFoundError(PathBuf, String),
    ProjectNotFoundError(PathBuf),
    PythonInterpreterError(pythons::Error),
    SystemEnvironmentError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::CommandNotFoundError(ref name) => {
                write!(f, "command {:?} not found", name)
            },
            Error::EnvironmentNotFoundError(ref root, ref name) => {
                write!(f, "environment not found for {:?} in {:?}", name, root)
            },
            Error::ProjectNotFoundError(ref p) => {
                write!(f, "project not found in {:?}", p)
            },
            Error::PythonInterpreterError(ref e) => e.fmt(f),
            Error::SystemEnvironmentError(ref e) => e.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::SystemEnvironmentError(e)
    }
}

impl From<pythons::Error> for Error {
    fn from(e: pythons::Error) -> Error {
        Error::PythonInterpreterError(e)
    }
}

type Result<T> = std::result::Result<T, Error>;


pub struct Project {
    interpreter: Interpreter,
    root: PathBuf,
}

impl Project {
    pub fn find(directory: &Path, interpreter: Interpreter) -> Result<Self> {
        let mut p = directory.canonicalize()?.to_path_buf();
        loop {
            if !p.is_dir() {
                continue;
            }
            if p.join("__pypackages__").is_dir() {
                return Ok(Self { root: p, interpreter: interpreter });
            }
            // TODO: Should we also look for other project markers like
            // pyproject.toml, Pipfile, etc.?
            if !p.pop() {
                break;
            }
        }
        Err(Error::ProjectNotFoundError(directory.to_path_buf()))
    }

    pub fn find_from_cwd(interpreter: Interpreter) -> Result<Self> {
        Self::find(&current_dir()?, interpreter)
    }

    fn pypackages(&self) -> PathBuf {
        self.root.join("__pypackages__")
    }

    fn site_packages(&self) -> Result<PathBuf> {
        let p = self.interpreter.presumed_site_packages(&self.pypackages())?;
        if p.is_dir() {
            Ok(p)
        } else {
            Err(Error::EnvironmentNotFoundError(
                self.root.to_owned(), self.interpreter.name().to_owned(),
            ))
        }
    }

    pub fn run<I, S>(&self, command: &str, args: I) -> Result<ExitStatus>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        let p = self.site_packages()?;
        for (name, entry) in EntryPoints::new(&p) {
            if name == command {
                let code = unindent(&format!(
                    "
                    import sys
                    from {} import {}
                    if __name__ == '__main__':
                        sys.argv[0] = {:?}
                        sys.exit({}())
                    ",
                    entry.module(),
                    entry.function(),
                    name,
                    entry.function(),
                ));

                // TODO: On Windows we should honor the entry.gui flag. Maybe
                // we should find pythonw.exe during interpreter discovery?
                return self.interpreter.interpret(&code, &p, args)?
                    .status()
                    .map_err(Error::from);
            }
        }
        Err(Error::CommandNotFoundError(command.to_owned()))
    }

    pub fn py<I, S>(&self, args: I) -> Result<ExitStatus>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        self.interpreter.command(&self.site_packages()?)?
            .args(args)
            .status()
            .map_err(Error::from)
    }
}
