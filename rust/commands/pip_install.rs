use std::process;

use clap::ArgMatches;

use crate::projects::Project;
use crate::pythons::{self, Interpreter};
use super::{Error, Result};

pub struct Command<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> Command<'a> {
    pub fn new(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    fn args(&self) -> Vec<&str> {
        self.matches.values_of("args").unwrap_or_default().collect()
    }

    pub fn run(&self, interpreter: Interpreter) -> Result<()> {
        let project = Project::find_in_cwd(interpreter)?;
        let env = project.presumed_env_root().unwrap();
        let interpreter = project.base_interpreter().location();

        let cmd = interpreter.to_str().ok_or_else(|| {
            pythons::Error::PathRepresentationError(interpreter.to_owned())
        })?;
        let args = vec![
            "-m", "pip", "install",
            "--prefix", env.to_str().unwrap(),
            "--no-warn-script-location",
        ].into_iter().chain(self.args()).collect::<Vec<_>>();

        let code = process::Command::new(cmd)
            .args(args)
            .status()?
            .code()
            .unwrap_or(-1);
        if code == 0 {
            Ok(())
        } else {
            Err(Error::SubprocessExit(code))
        }
    }
}
