use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::iter::empty;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use unindent::unindent;
use which;

use crate::foreign::Foreign;
use crate::vendors;

#[derive(Debug)]
pub enum Error {
    LookupError(which::Error),
    InvocationError(io::Error),
    IncompatibleInterpreterError(String),
    PathRepresentationError(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::LookupError(ref e) => e.fmt(f),
            Error::InvocationError(ref e) => e.fmt(f),
            Error::IncompatibleInterpreterError(ref s) => {
                write!(f, "interpreter {:?} has no compatibility tags", s)
            },
            Error::PathRepresentationError(ref p) => {
                write!(f, "{:?} not representable", p)
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

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! path_to_str {
    ($path:expr) => {
        {
            let p = $path;
            p.to_str().ok_or_else(|| Error::PathRepresentationError(p.into()))?
        }
    }
}


pub struct Interpreter {
    name: String,
    location: PathBuf,

    comptagcache: Option<String>,
}

impl Interpreter {
    pub fn discover<I, S>(name: &str, program: S, args: I) -> Result<Self>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        let code = "from __future__ import print_function; \
                    import sys; print(sys.executable, end='')";
        let out = Command::new(&which::which(program)?)
            .env("PYTHONIOENCODING", "utf-8")
            .args(args)
            .arg("-c")
            .arg(code)
            .output()?;

        let location = PathBuf::from(String::from_utf8(out.stdout).unwrap());
        Ok(Self {
            name: name.to_string(),
            location,
            comptagcache: None,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn location(&self) -> &Path {
        &self.location
    }

    pub fn command(
        &self,
        io_encoding: Option<&str>,
        pkgs: &Path,
    ) -> Result<Command> {
        let mut cmd = Command::new(&self.location);
        if let Some(encoding) = io_encoding {
            cmd.env("PYTHONIOENCODING", encoding);
        }
        cmd.env("PYTHONPATH", path_to_str!(pkgs));
        Ok(cmd)
    }

    fn interpret<I, S>(
        &self,
        encoding: Option<&str>,
        code: &str,
        pkgs: &Path,
        args: I,
    ) -> Result<Command>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        let mut cmd = self.command(encoding, pkgs)?;
        cmd.arg("-c");
        cmd.arg(&code);
        cmd.args(args);
        Ok(cmd)
    }

    pub fn create_venv(&self, env_dir: &Path, prompt: &str) -> Result<()> {
        let tmp_dir = TempDir::new()?;
        vendors::VirtEnv::populate_to(tmp_dir.path())?;

        let code = format!(
            "import virtenv; virtenv.create(\
             python=None, env_dir={:?}, prompt={:?},\
             system=False, bare=True)",
            path_to_str!(env_dir),
            prompt,
        );

        // TODO: Show message based on status code.
        let _status = self.interpret(
            None,
            &code,
            tmp_dir.path(),
            empty::<&str>(),
        )?.status()?;
        Ok(())
    }

    pub fn compatibility_tag(&self) -> Result<String> {
        if let Some(ref s) = self.comptagcache {
            return Ok(s.to_string());
        }

        let tmp_dir = TempDir::new()?;
        vendors::Pep425::populate_to(tmp_dir.path())?;

        let out = self.interpret(
            Some("utf-8"),
            "from __future__ import print_function; \
             import pep425; print(next(pep425.sys_tags()), end='')",
            tmp_dir.path(),
            empty::<&str>(),
        )?.output()?;

        // TODO: Show error if out.status() is not OK.

        let val = String::from_utf8(out.stdout).unwrap();
        if val.is_empty() {
            Err(Error::IncompatibleInterpreterError(self.name.to_owned()))
        } else {
            Ok(val)
        }
    }

    pub fn presumed_env_root(&self, pypackages: &Path) -> Result<PathBuf> {
        Ok(pypackages.join(self.compatibility_tag()?))
    }

    pub fn presumed_site_packages(
        &self,
        pypackages: &Path,
    ) -> Result<PathBuf> {
        let env_dir = self.presumed_env_root(pypackages)?;

        if cfg!(windows) {
            return Ok(env_dir.join("Lib").join("site-packages"));
        }

        let out = Command::new(&self.location)
            .env("PYTHONIOENCODING", "utf-8")
            .arg("-c")
            .arg("from __future__ import print_function; \
                  import sys; \
                  print('python{}.{}'.format(*sys.version_info), end='')")
            .output()?;

        // TODO: Show error if out.status() is not OK.

        let name = String::from_utf8(out.stdout).unwrap();
        Ok(env_dir.join("lib").join(&name).join("site-packages"))
    }

    fn run_molt_helper(&self, code: &str) -> Result<Option<i32>> {
        let tmp_dir = TempDir::new()?;
        vendors::Molt::populate_to(tmp_dir.path())?;

        let retcode = self.interpret(
            Some("utf-8"),
            code,
            tmp_dir.path(),
            empty::<&str>(),
        )?.status()?.code();
        Ok(retcode)
    }

    pub fn convert_foreign_lock(
        &self,
        foreign: Foreign,
        output: &Path,
    ) -> Result<i32> {
        let code = unindent(&match foreign {
            Foreign::PipfileLock(ref p) => format!(
                "
                import io
                import molt.pipfile_lock
                import plette
                with io.open({:?}, encoding='utf-8') as f:
                    pipfile_lock = plette.Lockfile.load(f)
                lockfile = molt.pipfile_lock.to_lock_file(pipfile_lock)
                with io.open({:?}, 'w', encoding='utf-8') as f:
                    lockfile.dump(f)
                ",
                path_to_str!(p),
                path_to_str!(output),
            ),
            Foreign::PoetryLock(ref p) => format!(
                "
                import io
                import molt.poetry_lock
                with io.open({:?}, encoding='utf-8') as f:
                    poetry_lock = molt.poetry_lock.load(f)
                lockfile = molt.poetry_lock.to_lock_file(poetry_lock)
                with io.open({:?}, 'w', encoding='utf-8') as f:
                    lockfile.dump(f)
                ",
                path_to_str!(p),
                path_to_str!(output),
            ),
        });

        Ok(self.run_molt_helper(&code)?.unwrap_or(-1))
    }
}
