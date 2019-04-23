mod cmd;
mod init;
mod pip_install;
mod py;
mod run;
mod show;
mod sync;

pub use self::cmd::{Error, Result};

use clap::ArgMatches;
use crate::pythons::{self, Interpreter};

macro_rules! subcommand {
    ($matches:expr, $module:ident) => {
        {
            let interpreter = discover_interpreter(&$matches)?;
            let n = stringify!($module).replace('_', "-");
            let matches = $matches.subcommand_matches(&n).unwrap();
            $module::Command::new(matches).run(interpreter)
        }
    };
}

fn discover_interpreter<'a>(matches: &'a ArgMatches) -> Result<Interpreter> {
    let py = matches.value_of("py").expect("required");
    let (prog, args) = if py.starts_with('-') {
        ("py", vec![py])
    } else {
        (py, vec![])
    };
    pythons::Interpreter::discover(py, prog, args).map_err(Error::from)
}

pub fn dispatch() -> Result<()> {
    let matches = cmd::app().get_matches();
    match matches.subcommand_name() {
        Some("init") => subcommand!(matches, init),
        Some("py") => subcommand!(matches, py),
        Some("run") => subcommand!(matches, run),
        Some("show") => subcommand!(matches, show),
        Some("sync") => subcommand!(matches, sync),

        Some("pip-install") => subcommand!(matches, pip_install),
        Some(n) => Err(Error::UnrecognizedSubcommand(n.to_string())),
        None => Err(Error::SubCommandMissing),
    }
}
