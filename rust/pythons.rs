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
                const N: &str = env!("CARGO_PKG_NAME");
                write!(f, "interpreter {:?} not compatible for {}", s, N)
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

    // Self cache to avoid repeated querying of compatibility tag.
    comptagcache: Option<String>,
}

impl Interpreter {
    fn new<S>(name: S, location: PathBuf) -> Self
        where S: Into<String>
    {
        Self { name: name.into(), location, comptagcache: None }
    }

    pub fn discover<I, S>(name: &str, program: S, args: I) -> Result<Self>
        where I: IntoIterator<Item=S>, S: AsRef<OsStr>
    {
        // TODO: Remove pip dependency check after we implement out own
        // package installing logic.
        let code = "from __future__ import print_function; import pip; \
                    import sys; print(sys.executable, end='')";
        let out = Command::new(&which::which(program)?)
            .env("PYTHONIOENCODING", "utf-8")
            .args(args)
            .arg("-c")
            .arg(code)
            .output()?;

        if out.status.success() {
            let loc = PathBuf::from(String::from_utf8(out.stdout).unwrap());
            Ok(Self::new(name, loc))
        } else {
            Err(Error::IncompatibleInterpreterError(name.to_owned()))
        }
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

    // This extra function is so tests can silence warnings, but the interface
    // can stay clean.
    fn convert_foreign_lock_impl(
        &self,
        foreign: Foreign,
        output: &Path,
        quiet: bool,
    ) -> Result<i32> {
        // Silence all warnings from Python.
        // This needs to be in one line, otherwise unindent breaks.
        static QUIET_CODE: &str = "import warnings; \
            warnings.formatwarning = lambda *_, **__: ''";

        let code = unindent(&match foreign {
            Foreign::PipfileLock(ref p) => format!(
                "
                import io
                import molt.foreign.pipfile_lock
                import plette
                {}
                with io.open({:?}, encoding='utf-8') as f:
                    pipfile_lock = plette.Lockfile.load(f)
                lockfile = molt.foreign.pipfile_lock.to_lock_file(pipfile_lock)
                with io.open({:?}, 'w', encoding='utf-8') as f:
                    lockfile.dump(f)
                ",
                if quiet { QUIET_CODE } else { "" },
                path_to_str!(p),
                path_to_str!(output),
            ),
            Foreign::PoetryLock(ref p) => format!(
                "
                import io
                import molt.foreign.poetry_lock
                {}
                with io.open({:?}, encoding='utf-8') as f:
                    poetry_lock = molt.foreign.poetry_lock.load(f)
                lockfile = molt.foreign.poetry_lock.to_lock_file(poetry_lock)
                with io.open({:?}, 'w', encoding='utf-8') as f:
                    lockfile.dump(f)
                ",
                if quiet { QUIET_CODE } else { "" },
                path_to_str!(p),
                path_to_str!(output),
            ),
        });

        let tmp_dir = TempDir::new()?;
        vendors::Molt::populate_to(tmp_dir.path())?;

        let mut cmd = self.interpret(
            Some("utf-8"),
            &code,
            tmp_dir.path(),
            empty::<&str>(),
        )?;
        Ok(cmd.status()?.code().unwrap_or(-1))
    }

    #[inline]
    pub fn convert_foreign_lock(
        &self,
        foreign: Foreign,
        output: &Path,
    ) -> Result<i32> {
        self.convert_foreign_lock_impl(foreign, output, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;
    use serde_json::from_str;
    use tempfile::NamedTempFile;

    struct Interpreters(Option<std::fs::ReadDir>);

    impl Iterator for Interpreters {
        type Item = Interpreter;
        fn next(&mut self) -> Option<Self::Item> {
            loop {
                let env = self.0.as_mut()?.next()?.ok()?.path();
                let exe = if cfg!(windows) {
                    env.join("Scripts").join("python.exe")
                } else {
                    env.join("bin").join("python")
                };
                if exe.is_file() {
                    let name = env.file_name().unwrap().to_string_lossy();
                    return Some(Interpreter::new(name, exe));
                }
            }
        }
    }

    fn find_interpreters() -> Interpreters {
        let tox_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tox");
        Interpreters(tox_dir.read_dir().ok())
    }

    #[test]
    fn test_convert_foreign_lock() {
        let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");

        for interpreter in find_interpreters() {
            let dirs = samples.read_dir().expect("cannot read samples");
            for dir in dirs {
                let dir = dir.expect("cannot read sample").path();
                let foreign = match Foreign::find_in(&dir) {
                    Some(f) => f,
                    None => { continue; },
                };

                let real_out = NamedTempFile::new().unwrap().into_temp_path();

                let result = interpreter.convert_foreign_lock_impl(
                    foreign, &real_out, true,
                );
                assert_eq!(result.unwrap(), 0);

                let expected = dir.join("molt.lock.json");
                assert_json_eq!(
                    from_str(&read_to_string(&real_out).unwrap()).unwrap(),
                    from_str(&read_to_string(&expected).unwrap()).unwrap(),
                );
            }
        }
    }
}
