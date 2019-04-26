use std::{fmt, io};

use clap::{App, AppSettings, Arg, SubCommand};
use which::which;

use crate::{projects, pythons, sync};

pub fn app<'a, 'b>() -> App<'a, 'b> {
    let py_available = which("py").is_ok();

    app_from_crate!()
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("py")
            .long("py")
            .help("Python interpreter to use")
            .required(true)
            .takes_value(true)
            .allow_hyphen_values(py_available)
        )
        .subcommand(SubCommand::with_name("show")
            .about("Print project information")
            .setting(AppSettings::ArgRequiredElseHelp)
            .arg(Arg::with_name("env")
                .long("env")
                .help("Path to the environment")
            )
        )
        .subcommand(SubCommand::with_name("init")
            .about("Initialize an environment for project")
            .arg(Arg::with_name("project")
                .help("Path to project root directory")
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("sync")
            .about("Synchronize environment with locked project dependencies")
            .arg(Arg::with_name("no_default")
                .long("--no-default")
                .help("Do no install the default section")
                .requires("extras")
            )
            .arg(Arg::with_name("extras")
                .long("--with")
                .help("Extra sections to install")
                .value_delimiter(",")
            )
        )
        .subcommand(SubCommand::with_name("run")
            .about("Run a command in the environment")
            .setting(AppSettings::AllowLeadingHyphen)
            .setting(AppSettings::DisableHelpFlags)
            .arg(Arg::with_name("command")
                .help("Command to run")
                .required(true)
            )
            .arg(Arg::with_name("args")
                .help("Arguments to command")
                .multiple(true)
            )
        )
        .subcommand(SubCommand::with_name("py")
            .about("Run the Python interpreter in the environment")
            .setting(AppSettings::AllowLeadingHyphen)
            .setting(AppSettings::DisableHelpFlags)
            .arg(Arg::with_name("args")
                .help("Arguments to interpreter")
                .multiple(true)
            )
        )
        .subcommand(SubCommand::with_name("convert")
            .about("Convert a foreign lock file format to molt.lock.json")
        )
        .subcommand(SubCommand::with_name("pip-install")
            .about("Secret subcommand to install things into the environment")
            .setting(AppSettings::AllowLeadingHyphen)
            .setting(AppSettings::DisableHelpFlags)
            .setting(AppSettings::Hidden)
            .arg(Arg::with_name("args")
                .help("Arguments to pip install")
                .multiple(true)
            )
        )
}

#[derive(Debug)]
pub enum Error {
    ConvertError(i32),
    InterpreterError(pythons::Error),
    ProjectError(projects::Error),
    SubCommandMissing,
    SubprocessExit(i32),
    SyncError(sync::Error),
    SystemError(io::Error),
    UnrecognizedSubcommand(String),
}

impl Error {
    pub fn status(&self) -> i32 {
        match *self {
            // Bridged error from subprocess.
            Error::SubprocessExit(v) => v,

            // General command errors.
            Error::ConvertError(_) => 1,
            Error::SyncError(_) => 2,

            // Can't run without a project ._.
            Error::ProjectError(_) => 0x10_00_00_01,

            // Shouldn't happen unless there's a bug in Clap.
            Error::SubCommandMissing => 0x60_00_00_01,
            Error::UnrecognizedSubcommand(_) => 0x60_00_00_02,

            // Something is very wrong in the user's runtime environment.
            Error::InterpreterError(_) => 0x70_00_00_01,
            Error::SystemError(_) => 0x70_00_00_02,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ConvertError(c) => {
                write!(f, "conversion failed with error {}", c)
            },
            Error::InterpreterError(ref e) => e.fmt(f),
            Error::ProjectError(ref e) => e.fmt(f),
            Error::SubCommandMissing => write!(f, "missing subcommand"),
            Error::SubprocessExit(c) => {
                write!(f, "process exited with status code {}", c)
            },
            Error::SyncError(ref e) => e.fmt(f),
            Error::SystemError(ref e) => e.fmt(f),
            Error::UnrecognizedSubcommand(ref n) => {
                write!(f, "unhandled subcommand {:?}", n)
            },
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::SystemError(e)
    }
}

impl From<projects::Error> for Error {
    fn from(e: projects::Error) -> Self {
        Error::ProjectError(e)
    }
}

impl From<pythons::Error> for Error {
    fn from(e: pythons::Error) -> Self {
        Error::InterpreterError(e)
    }
}

impl From<sync::Error> for Error {
    fn from(e: sync::Error) -> Self {
        Error::SyncError(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
