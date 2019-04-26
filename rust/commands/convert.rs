use clap::ArgMatches;
use unindent::unindent;

use crate::projects::{ForeignFile, Project};
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

        let lock_file_path = project.persumed_lock_file_path();

        let foreign_file_path = match project.foreign_lock_file_path()? {
            ForeignFile::PipfileLock(p) => p,
        };

        let code = unindent(&format!(
            "
            import io
            import molt.pipfile_lock
            import plette
            with io.open({:?}, encoding='utf-8') as f:
                pipfile_lock = plette.Lockfile.load(f)
            lockfile = molt.pipfile_lock.to_lock_file(pipfile_lock)
            with io.open({:?}, 'w', encoding='utf-8') as f:
                lockfile.dump(f)
            ",
            foreign_file_path,
            lock_file_path,
        ));

        let retcode = project.base_interpreter()
            .run_molt_helper(&code)?
            .unwrap_or(-1);
        if retcode == 0 {
            Ok(())
        } else {
            Err(Error::ConvertError(retcode))
        }
    }
}
