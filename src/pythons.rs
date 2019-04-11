use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Interpreter {
    location: PathBuf,
}

macro_rules! command {
    ($self:ident) => (
        Command::new(&$self.location).env("PYTHONIOENCODING", "utf-8")
    )
}

impl Interpreter {
    fn from(location: &Path) -> Self {
        Self {
            location: location.to_owned(),
        }
    }

    fn sys_executable(&self) -> Result<PathBuf> {
        let output = command!(self).args(&[
            "-c",
            "from __future__ import print_function; \
             import sys; print(sys.executable, end='')",
        ]).output()?;

        // This cannot fail because we told the interpreter to use UTF-8.
        Ok(PathBuf::from(String::from_utf8(output.stdout).unwrap()))
    }


}

#[cfg(test)]
mod tests {
    extern crate which;
    use super::*;
    use self::which::which;

    #[test]
    fn test_interpreter() {
        let int = Interpreter::from(&which("python").unwrap());
        assert!(int.sys_executable().is_ok(), "{:?}", int.sys_executable());
    }
}
