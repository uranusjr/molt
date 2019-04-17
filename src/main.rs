#[macro_use] extern crate clap;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate prettytable;
#[macro_use] extern crate rust_embed;

extern crate dunce;
extern crate ini;
extern crate regex;
extern crate tempdir;
extern crate unindent;
extern crate which;

mod args;
mod entrypoints;
mod projects;
mod pythons;
mod vendors;

use prettytable::format::consts::*;

fn main() {
    let opts = args::Options::new();
    let interpreter = opts.interpreter().expect("interpreter unavailable");

    match opts.sub_options() {
        args::Sub::None => {},
        args::Sub::Show(show_opts) => {
            let project = projects::Project::find_from_cwd(interpreter)
                .expect("TODO: Fail gracefully when project is not found.");
            match show_opts.what() {
                args::ShowWhat::Env => {
                    let env = project.presumed_env_root().unwrap();
                    println!("{}", dunce::simplified(&env).display());
                },
            }
            std::process::exit(0);
        },
        args::Sub::Init(init_opts) => {
            let envdir = init_opts.project_root()
                .join("__pypackages__")
                .join(interpreter.compatibility_tag()
                    .expect("TODO: Fail gracefully if Python call fails."));
            let prompt = init_opts.project_name()
                .unwrap_or_else(|| String::from("venv"));
            interpreter.create_venv(&envdir, &prompt)
                .expect("Cannot create venv");
        },
        args::Sub::Run(run_opts) => {
            let project = projects::Project::find_from_cwd(interpreter)
                .expect("TODO: Fail gracefully when project is not found.");
            let command = run_opts.command();
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
                std::process::exit(0);
            } else {
                let status = project.run(command, run_opts.args())
                    .expect("TODO: Fail gracefully when run fails.");
                // TODO: What error should we use if the command cannot run?
                std::process::exit(status.code().unwrap_or(-1));
            }
        },
        args::Sub::Py(py_opts) => {
            let project = projects::Project::find_from_cwd(interpreter)
                .expect("TODO: Fail gracefully when project is not found.");
            let status = project.py(py_opts.args())
                .expect("TODO: Fail gracefully when py fails.");
            // TODO: What error should we use if the interpreter cannot start?
            std::process::exit(status.code().unwrap_or(-1));
        },
        args::Sub::PipInstall(pip_install_opts) => {
            let project = projects::Project::find_from_cwd(interpreter)
                .expect("TODO: Fail gracefully when project is not found.");
            let env = project.presumed_env_root().unwrap();
            let args =
                vec![
                    "-m", "pip", "install",
                    "--prefix", dunce::simplified(&env).to_str().unwrap(),
                    "--no-warn-script-location",
                ]
                .into_iter()
                .chain(pip_install_opts.args())
                .collect::<Vec<&str>>();
            let cmd = project.base_interpreter().location().to_str().unwrap();
            let status = std::process::Command::new(cmd)
                .args(args)
                .status()
                .expect("TODO: Fail gracefully when py fails.");
            // TODO: What error should we use if the interpreter cannot start?
            std::process::exit(status.code().unwrap_or(-1));
        },
    }
}
