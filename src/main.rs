#[macro_use] extern crate clap;
#[macro_use] extern crate rust_embed;

extern crate shlex;
extern crate tempdir;
extern crate which;

mod pythons;
mod vendors;

fn venv_prompt() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    Some(cwd.file_name()?.to_string_lossy().into_owned())
}

fn main() {
    let opts = app_from_crate!()
        .arg(clap::Arg::with_name("py_cmd")
            .long("py")
            .help("Python interpreter to use")
            .required(true)
            .takes_value(true)
        )
        .arg(clap::Arg::with_name("env_dir")
            .long("to")
            .help("Directory to create environment at")
            .required(true)
            .takes_value(true)
        )
        .get_matches();

    let py_cmd = opts.value_of("py_cmd").expect("required arg");

    let py_args = shlex::split(py_cmd).unwrap();
    let python = pythons::Interpreter::discover(
        py_args.get(0).unwrap(),
        py_args.get(1..).unwrap(),
    ).expect("Python not found");

    python.create_venv(
        &std::path::Path::new(opts.value_of("env_dir").expect("required arg")),
        &venv_prompt().unwrap_or(String::from("venv")),
    ).expect("Cannot create venv");
}
