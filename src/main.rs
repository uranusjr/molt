#[macro_use] extern crate clap;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate prettytable;
#[macro_use] extern crate rust_embed;
#[macro_use] extern crate serde;

extern crate dunce;
extern crate ini;
extern crate regex;
extern crate serde_json;
extern crate tempfile;
extern crate unindent;
extern crate url;
extern crate which;

mod commands;
mod entrypoints;
mod lockfiles;
mod projects;
mod pythons;
mod sync;
mod vendors;

fn main() {
    if let Err(e) = commands::dispatch() {
        eprintln!("{}", e);
        std::process::exit(e.status());
    }
}
