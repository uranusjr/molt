use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::iter::empty;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempdir::TempDir;
use which;

use crate::vendors;

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

pub type Result<T> = std::result::Result<T, Error>;


fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd
}


#[allow(dead_code)]
pub struct Interpreter {
    name: String,
    location: PathBuf,

    comptagcache: Option<String>,
}

impl Interpreter {
    pub fn discover<I, S>(name: &str, program: S, args: I) -> Result<Self>
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

        let location = PathBuf::from(String::from_utf8(out.stdout).unwrap());
        Ok(Self {
            name: name.to_string(),
            location: location,
            comptagcache: None,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command(&self, pkgs: &Path) -> Result<Command> {
        let pythonpath = pkgs.to_str().ok_or(Error::UnrepresentableError)?;
        let mut cmd = command(&self.location);
        cmd.env("PYTHONPATH", pythonpath);
        Ok(cmd)
    }

    pub fn interpret<I, S>(
        &self,
        code: &str,
        pkgs: &Path,
        args: I,
    ) -> Result<Command>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        let mut cmd = self.command(pkgs)?;
        cmd.arg("-c");
        cmd.arg(&code);
        cmd.args(args);
        Ok(cmd)
    }

    pub fn create_venv(&self, env_dir: &Path, prompt: &str) -> Result<()> {
        let tmp_dir = TempDir::new("molt-virtenv")?;
        vendors::VirtEnv::populate_to(tmp_dir.path())?;

        let code = format!(
            "import virtenv; virtenv.create(None, {:?}, False, prompt={:?})",
            env_dir.to_string_lossy().into_owned(),
            prompt,
        );

        // TODO: Show message based on status code.
        let _status = self.interpret(
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

        let tmp_dir = TempDir::new("molt-pep425")?;
        vendors::Pep425::populate_to(tmp_dir.path())?;

        let out = self.interpret(
            "from __future__ import print_function; \
             import pep425; print(next(pep425.sys_tags()), end='')",
            tmp_dir.path(),
            empty::<&str>(),
        )?.output()?;

        // TODO: Show error if out.status() is not OK.

        let val = String::from_utf8(out.stdout).unwrap();
        if val.is_empty() {
            Err(Error::UnrepresentableError)
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
            let p = env_dir.join("Lib").join("site-packages");
            return Ok(p.to_path_buf());
        }

        let out = command(&self.location)
            .arg("-c")
            .arg("from __future__ import print_function; \
                  import sys; \
                  print('python{}.{}'.format(*sys.version_info), end='')")
            .output()?;

        // TODO: Show error if out.status() is not OK.

        let name = String::from_utf8(out.stdout).unwrap();
        Ok(env_dir.join("lib").join(&name).join("site-packages"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_discover() {
        let result = Interpreter::discover("python", &"python", &[]);
        assert!(result.is_ok());
    }
}
