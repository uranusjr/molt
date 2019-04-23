#![allow(dead_code, unused_variables)]

use clap::{ArgMatches, Values};

use crate::projects::Project;
use crate::pythons::Interpreter;
use crate::sync::Synchronizer;
use super::Result;

pub struct Command<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> Command<'a> {
    pub fn new(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    fn default(&self) -> bool {
        !self.matches.is_present("no_default")
    }

    fn extras(&self) -> Values {
        self.matches.values_of("extras").unwrap_or_default()
    }

    pub fn run(&self, interpreter: Interpreter) -> Result<()> {
        let project = Project::find_in_cwd(interpreter)?;
        let sync = Synchronizer::new(project.read_lock_file()?)?;
        sync.sync(&project, self.default(), self.extras())?;
        Ok(())
    }
}
