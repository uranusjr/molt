#[macro_use] extern crate clap;
#[macro_use] extern crate rust_embed;

extern crate ini;
extern crate tempdir;
extern crate which;

mod args;
mod entrypoints;
mod pythons;
mod vendors;

use args::Sub;

fn main() {
    let opts = args::Options::new();
    let interpreter = opts.interpreter().expect("interpreter unavailable");

    match opts.sub_options() {
        Sub::Init(init_opts) => {
            let mut envdir = init_opts.project_root();
            envdir.push("__pypackages__");
            envdir.push(interpreter.compatibility_tag().unwrap());
            let prompt = init_opts.project_name()
                .unwrap_or(String::from("venv"));
            interpreter.create_venv(&envdir, &prompt)
                .expect("Cannot create venv");
        },
        Sub::None => {},
    }
}
