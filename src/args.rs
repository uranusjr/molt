use std::path::PathBuf;

use clap::{App, Arg, ArgMatches, SubCommand};
use shlex;

use crate::pythons::{self, Interpreter};

fn app<'a, 'b>() -> App<'a, 'b> {
    app_from_crate!()
        .arg(Arg::with_name("py_cmd")
            .long("py")
            .help("Python interpreter to use")
            .required(true)
            .takes_value(true)
        )
        .subcommand(SubCommand::with_name("init")
            .about("Initialize an environment for project")
            .arg(Arg::with_name("project_root")
                .help("Path to project root directory")
            )
        )
}

pub enum Sub<'a> {
    None,
    Init(InitOptions<'a>),
}

pub struct Options<'a> {
    matches: ArgMatches<'a>,
}

impl<'a> Options<'a> {
    pub fn new() -> Self {
        Self { matches: app().get_matches() }
    }

    pub fn interpreter(&self) -> pythons::Result<Interpreter> {
        let cmd = self.matches.value_of("py_cmd").expect("required");
        let args = shlex::split(cmd).unwrap_or_else(|| vec![String::new()]);
        Interpreter::discover(args.get(0).unwrap(), args.get(1..).unwrap())
    }

    pub fn sub_options(&self) -> Sub {
        match self.matches.subcommand_name() {
            Some("init") => Sub::Init(InitOptions::new(&self.matches)),
            _ => Sub::None,
        }
    }
}

pub struct InitOptions<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> InitOptions<'a> {
    fn new(parent: &'a ArgMatches) -> Self {
        Self { matches: parent.subcommand_matches("init").unwrap() }
    }

    pub fn project_root(&self) -> PathBuf {
        PathBuf::from(self.matches.value_of("project_root").expect("required"))
    }

    pub fn project_name(&self) -> Option<String> {
        let root = self.project_root();
        let root = root.canonicalize().unwrap_or(root);
        root.file_name().map(|n| n.to_string_lossy().into_owned())
    }
}
