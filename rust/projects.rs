use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use dunce;
use serde_json;
use unindent::unindent;

use crate::entrypoints::EntryPoints;
use crate::foreign::Foreign;
use crate::lockfiles::Lock;
use crate::pythons::{self, Interpreter};

#[derive(Debug)]
pub enum Error {
    CommandNotFoundError(String),
    EnvironmentNotFoundError(PathBuf, String),
    EnvironmentSetupError(env::JoinPathsError),
    ForeignLockFileNotFoundError(PathBuf),
    LockFileNotFoundError(PathBuf),
    LockFileInvalidError(serde_json::Error),
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
            Error::ForeignLockFileNotFoundError(ref p) => {
                write!(f, "foreign lock file not found in directory {:?}", p)
            },
            Error::LockFileNotFoundError(ref p) => {
                write!(f, "lock file expected but not found at {:?}", p)
            },
            Error::LockFileInvalidError(ref e) => e.fmt(f),
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

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::LockFileInvalidError(e)
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
        let mut p = dunce::canonicalize(directory)?;
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

    pub fn find_in_cwd(interpreter: Interpreter) -> Result<Self> {
        Self::find(&env::current_dir()?, interpreter)
    }

    // TODO: We might be able to remove this after removing pip-install.
    pub fn base_interpreter(&self) -> &Interpreter {
        &self.interpreter
    }

    pub fn persumed_lock_file_path(&self) -> PathBuf {
        self.root.join("molt.lock.json")
    }

    pub fn read_lock_file(&self) -> Result<Lock> {
        let p = self.persumed_lock_file_path();
        if p.is_file() {
            Ok(serde_json::from_reader(BufReader::new(File::open(p)?))?)
        } else {
            Err(Error::LockFileNotFoundError(p))
        }
    }

    pub fn command(&self, io_encoding: Option<&str>) -> Result<Command> {
        self.interpreter
            .command(io_encoding, &self.site_packages()?)
            .map_err(Error::from)
    }

    fn persumed_pypackages(&self) -> PathBuf {
        self.root.join("__pypackages__")
    }

    pub fn presumed_env_root(&self) -> Result<PathBuf> {
        let pypackages = self.persumed_pypackages();
        self.interpreter.presumed_env_root(&pypackages).map_err(Error::from)
    }

    pub fn env_root(&self) -> Result<PathBuf> {
        let p = self.presumed_env_root()?;
        if p.is_dir() {
            Ok(p)
        } else {
            Err(Error::EnvironmentNotFoundError(
                self.root.to_owned(), self.interpreter.name().to_owned(),
            ))
        }
    }

    fn site_packages(&self) -> Result<PathBuf> {
        let pypackages = self.persumed_pypackages();
        let p = self.interpreter.presumed_site_packages(&pypackages)?;
        if p.is_dir() {
            Ok(p)
        } else {
            Err(Error::EnvironmentNotFoundError(
                self.root.to_owned(), self.interpreter.name().to_owned(),
            ))
        }
    }

    #[allow(dead_code)]
    fn bindir(&self) -> Result<PathBuf> {
        #[cfg(target_os = "windows")] static BINDIR_NAME: &str = "Scripts";
        #[cfg(not(target_os = "windows"))] static BINDIR_NAME: &str = "bin";

        let p = self.presumed_env_root()?.join(BINDIR_NAME);
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

        // TODO: Is this a good idea? I don't think so since the executables
        // in the environment aren't really meant to be used. They might not
        // even be compatible if they are created on another machine (with the
        // same architecture).
        // cmd.env("PATH", {
        //     let p = env::var("PATH").unwrap_or_default();
        //     let chained = iter::once(self.bindir()?).to_owned())
        //         .chain(env::split_paths(&p));
        //     env::join_paths(chained)?
        // });

        // I *think* this is OK? Some tools sniff it, so it might be better to
        // say we are (an equivalent of) a virtual environment.
        cmd.env("VIRTUAL_ENV", self.presumed_env_root()?);

        // HACK: pip sniffs sys.real_prefix and sys.base_prefix to detect
        // whether it's in a virtual environment, and barks if the user sets
        // this to true. I can't find another realiable way around it.
        cmd.env("PIP_REQUIRE_VIRTUALENV", "false");

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

    pub fn convert_foreign_lock(&self) -> Result<i32> {
        Ok(self.interpreter.convert_foreign_lock(
            Foreign::find_in(&self.root).ok_or_else(|| {
                Error::ForeignLockFileNotFoundError(self.root.to_owned())
            })?,
            &self.persumed_lock_file_path(),
        )?)
    }
}
