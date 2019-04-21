use clap::ArgMatches;

use crate::projects::Project;
use crate::pythons::Interpreter;
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
        let code = project.py(self.args())?.code().unwrap_or(-1);
        if code == 0 {
            Ok(())
        } else {
            Err(Error::SubprocessExit(code))
        }
    }
}
