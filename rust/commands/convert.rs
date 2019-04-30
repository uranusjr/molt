use clap::ArgMatches;

use crate::projects::Project;
use crate::pythons::Interpreter;
use super::{Error, Result};

pub struct Command<'a> {
    _matches: &'a ArgMatches<'a>,
}

impl<'a> Command<'a> {
    pub fn new(_matches: &'a ArgMatches) -> Self {
        Self { _matches }
    }

    pub fn run(&self, interpreter: Interpreter) -> Result<()> {
        let project = Project::find_in_cwd(interpreter)?;

        let code = project.convert_foreign_lock()?;

        if code == 0 {
            Ok(())
        } else {
            Err(Error::ConvertError(code))
        }
    }
}
