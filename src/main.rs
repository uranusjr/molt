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

mod commands;
mod entrypoints;
mod projects;
mod pythons;
mod vendors;

fn main() {
    if let Err(e) = commands::dispatch() {
        eprintln!("{}", e);
        std::process::exit(e.status());
    }
}
