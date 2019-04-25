use clap::ArgMatches;

use crate::projects::Project;
use crate::pythons::Interpreter;
use super::Result;

pub enum What {
    Env,
}

pub struct Command<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> Command<'a> {
    pub fn new(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    fn what(&self) -> What {
        if self.matches.is_present("env") {
            What::Env
        } else {
            panic!("one of the options should present");
        }
    }

    pub fn run(&self, interpreter: Interpreter) -> Result<()> {
        let project = Project::find_in_cwd(interpreter)?;
        match self.what() {
            What::Env => {
                let env = project.presumed_env_root().unwrap();
                println!("{}", env.display());
            },
        }
        Ok(())
    }
}
