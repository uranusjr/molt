#[macro_use] extern crate clap;
#[macro_use] extern crate rust_embed;

extern crate ini;
extern crate tempdir;
extern crate unindent;
extern crate which;

mod args;
mod entrypoints;
mod projects;
mod pythons;
mod vendors;

use args::Sub;

fn main() {
    let opts = args::Options::new();
    let interpreter = opts.interpreter().expect("interpreter unavailable");

    match opts.sub_options() {
        Sub::None => {},
        Sub::Init(init_opts) => {
            let envdir = init_opts.project_root()
                .join("__pypackages__")
                .join(interpreter.compatibility_tag()
                    .expect("TODO: Fail gracefully if Python call fails."));
            let prompt = init_opts.project_name()
                .unwrap_or_else(|| String::from("venv"));
            interpreter.create_venv(&envdir, &prompt)
                .expect("Cannot create venv");
        },
        Sub::Run(run_opts) => {
            let project = projects::Project::find_from_cwd(interpreter)
                .expect("TODO: Fail gracefully when project is not found.");
            let status = project.run(run_opts.command(), run_opts.args())
                .expect("TODO: Fail gracefully when run fails.");

            // TODO: What error should we use if the command fails to execute?
            std::process::exit(status.code().unwrap_or(-1));
        },
        Sub::Py(py_opts) => {
            let project = projects::Project::find_from_cwd(interpreter)
                .expect("TODO: Fail gracefully when project is not found.");
            let status = project.py(py_opts.args())
                .expect("TODO: Fail gracefully when py fails.");

            // TODO: What error should we use if the interpreter cannot start?
            std::process::exit(status.code().unwrap_or(-1));
        },
    }
}
