use std::path::PathBuf;

use clap::ArgMatches;

use crate::pythons::Interpreter;
use super::Result;

pub struct Command<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> Command<'a> {
    pub fn new(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    fn project_root(&self) -> PathBuf {
        PathBuf::from(self.matches.value_of("project").expect("required"))
    }

    fn project_name(&self) -> Option<String> {
        let root = self.project_root();
        let root = root.canonicalize().unwrap_or(root);
        root.file_name().map(|n| n.to_string_lossy().into_owned())
    }

    pub fn run(&self, interpreter: Interpreter) -> Result<()> {
        let envdir = self.project_root()
            .join("__pypackages__")
            .join(interpreter.compatibility_tag()?);
        let prompt = self.project_name()
            .unwrap_or_else(|| String::from("venv"));
        interpreter.create_venv(&envdir, &prompt)?;
        Ok(())
    }
}
