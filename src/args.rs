use std::path::PathBuf;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use crate::pythons::{self, Interpreter};

fn app<'a, 'b>() -> App<'a, 'b> {
    app_from_crate!()
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("py")
            .long("py")
            .help("Python interpreter to use")
            .required(true)
            .takes_value(true)
            .allow_hyphen_values(true)
        )
        .subcommand(SubCommand::with_name("init")
            .about("Initialize an environment for project")
            .arg(Arg::with_name("project")
                .help("Path to project root directory")
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("run")
            .about("Run a command in the environment")
            .arg(Arg::with_name("command")
                .help("Command to run")
                .required(true)
            )
            .arg(Arg::with_name("args")
                .help("Arguments to command")
                .multiple(true)
                .allow_hyphen_values(true)  // Doesn't work (clap-rs/clap#1437)
            )
        )
        .subcommand(SubCommand::with_name("py")
            .about("Run the Python interpreter in the environment")
            .arg(Arg::with_name("args")
                .help("Arguments to interpreter")
                .multiple(true)
                .allow_hyphen_values(true)  // Doesn't work (clap-rs/clap#1437)
            )
        )
}

pub enum Sub<'a> {
    None,
    Init(InitOptions<'a>),
    Run(RunOptions<'a>),
    Py(PyOptions<'a>),
}

pub struct Options<'a> {
    matches: ArgMatches<'a>,
}

impl<'a> Options<'a> {
    pub fn new() -> Self {
        Self { matches: app().get_matches() }
    }

    pub fn interpreter(&self) -> pythons::Result<Interpreter> {
        let py = self.matches.value_of("py").expect("required");
        let (prog, args) = if py.starts_with("-") {
            ("py", vec![py])
        } else {
            (py, vec![])
        };
        Interpreter::discover(py, prog, args)
    }

    pub fn sub_options(&self) -> Sub {
        match self.matches.subcommand_name() {
            Some("init") => Sub::Init(InitOptions::new(&self.matches)),
            Some("run") => Sub::Run(RunOptions::new(&self.matches)),
            Some("py") => Sub::Py(PyOptions::new(&self.matches)),
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
        PathBuf::from(self.matches.value_of("project").expect("required"))
    }

    pub fn project_name(&self) -> Option<String> {
        let root = self.project_root();
        let root = root.canonicalize().unwrap_or(root);
        root.file_name().map(|n| n.to_string_lossy().into_owned())
    }
}

pub struct RunOptions<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> RunOptions<'a> {
    fn new(parent: &'a ArgMatches) -> Self {
        Self { matches: parent.subcommand_matches("run").unwrap() }
    }

    pub fn command(&self) -> &str {
        self.matches.value_of("command").expect("required")
    }

    pub fn args(&self) -> Vec<&str> {
        self.matches.values_of("args").unwrap_or_default().collect()
    }
}

pub struct PyOptions<'a> {
    matches: &'a ArgMatches<'a>,
}

impl<'a> PyOptions<'a> {
    fn new(parent: &'a ArgMatches) -> Self {
        Self { matches: parent.subcommand_matches("py").unwrap() }
    }

    pub fn args(&self) -> Vec<&str> {
        self.matches.values_of("args").unwrap_or_default().collect()
    }
}
