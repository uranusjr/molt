use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use dunce::simplified;
use unindent::unindent;

use crate::entrypoints::EntryPoints;
use crate::pythons::{self, Interpreter};

#[cfg(target_os = "windows")]
static BINDIR_NAME: &str = "Scripts";

#[cfg(not(target_os = "windows"))]
static BINDIR_NAME: &str = "bin";

#[derive(Debug)]
pub enum Error {
    CommandNotFoundError(String),
    EnvironmentNotFoundError(PathBuf, String),
    EnvironmentSetupError(env::JoinPathsError),
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
            Error::EnvironmentSetupError(ref e) => e.fmt(f),
            Error::ProjectNotFoundError(ref p) => {
                write!(f, "project not found in {:?}", p)
            },
            Error::PythonInterpreterError(ref e) => e.fmt(f),
            Error::SystemEnvironmentError(ref e) => e.fmt(f),
        }
    }
}

impl From<env::JoinPathsError> for Error {
    fn from(e: env::JoinPathsError) -> Error {
        Error::EnvironmentSetupError(e)
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
                return Ok(Self { root: p, interpreter });
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
        Self::find(&env::current_dir()?, interpreter)
    }

    fn pypackages(&self) -> PathBuf {
        self.root.join("__pypackages__")
    }

    pub fn presumed_env_root(&self) -> Result<PathBuf> {
        self.interpreter.presumed_env_root(&self.pypackages())
            .map_err(Error::from)
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

    fn bindir(&self) -> Result<PathBuf> {
        let p = self.interpreter.presumed_env_root(&self.pypackages())?
            .join(BINDIR_NAME);
        if p.is_dir() {
            Ok(p)
        } else {
            Err(Error::EnvironmentNotFoundError(
                self.root.to_owned(), self.interpreter.name().to_owned(),
            ))
        }
    }

    pub fn entry_points(&self) -> Result<EntryPoints> {
        Ok(EntryPoints::new(&(self.site_packages()?)))
    }

    fn run_interpreter(&self) -> Result<Command> {
        let mut cmd = self.interpreter.command(None, &self.site_packages()?)?;
        cmd.env("PATH", {
            let p = env::var_os("PATH").unwrap_or_default();
            let chained = iter::once(self.bindir()?)
                .chain(env::split_paths(&p));
            env::join_paths(chained)?
        });
        cmd.env("VIRTUAL_ENV", simplified(&self.presumed_env_root()?));
        Ok(cmd)
    }

    pub fn run<I, S>(&self, command: &str, args: I) -> Result<ExitStatus>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        for (name, entry) in EntryPoints::new(&self.site_packages()?) {
            if name == command {
                let function = entry.function();
                let code = unindent(&format!(
                    "
                    import sys
                    from {} import {}
                    if __name__ == '__main__':
                        sys.argv[0] = {:?}
                        sys.exit({}())
                    ",
                    entry.module(),
                    function.split('.').next().unwrap_or(function),
                    name,
                    function,
                ));

                // TODO: On Windows we should honor the entry.gui flag. Maybe
                // we should find pythonw.exe during interpreter discovery?
                return self.run_interpreter()?
                    .arg("-c")
                    .arg(&code)
                    .args(args)
                    .status()
                    .map_err(Error::from);
            }
        }
        Err(Error::CommandNotFoundError(command.to_owned()))
    }

    pub fn py<I, S>(&self, args: I) -> Result<ExitStatus>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        self.run_interpreter()?.args(args).status().map_err(Error::from)
    }
}
