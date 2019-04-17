#[macro_use] extern crate clap;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate prettytable;
#[macro_use] extern crate rust_embed;
#[macro_use] extern crate serde;

extern crate dunce;
extern crate ini;
extern crate regex;
extern crate serde_json;
extern crate tempdir;
extern crate unindent;
extern crate which;

mod commands;
mod entrypoints;
mod locks;
mod projects;
mod pythons;
mod vendors;

fn main() {
    match commands::dispatch() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(e.status());
        },
    }
}
