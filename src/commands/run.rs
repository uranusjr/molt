use clap::ArgMatches;
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR;

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

    fn command(&self) -> &str {
        self.matches.value_of("command").expect("required")
    }

    fn args(&self) -> Vec<&str> {
        self.matches.values_of("args").unwrap_or_default().collect()
    }

    pub fn run(&self, interpreter: Interpreter) -> Result<()> {
        let project = Project::find_in_cwd(interpreter)?;
        let command = self.command();
        if command == "--list" {
            // HACK: Handle "run --list".
            let mut eps: Vec<Vec<String>> = project.entry_points().unwrap()
                .map(|(n, e)| {
                    let call = format!("{}:{}", e.module(), e.function());
                    vec![n, call]
                })
                .collect();
            eps.sort_unstable();
            let mut table = prettytable::Table::from(eps);
            table.set_titles(row!["Entry point", "Call target"]);
            table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
            table.printstd();
            Ok(())
        } else {
            let code = project.run(command, self.args())?.code().unwrap_or(-1);
            if code == 0 {
                Ok(())
            } else {
                Err(Error::SubprocessExit(code))
            }
        }
    }
}
