extern crate which;

use std::env;
use std::path::Path;
use std::process::Command;

enum DependedFile<'a> {
    Requirements(&'a str),
    Script(&'a str),
}

static VENDOR_SCRIPT: &str = "__main__.py";

fn find_depended_file(p: &Path) -> Option<DependedFile> {
    // The vendor script.
    if p.file_name()? == VENDOR_SCRIPT {
        return Some(DependedFile::Script(VENDOR_SCRIPT));
    }

    // Requirements files.
    if p.extension()?.to_str()? != "txt" {
        return None;
    }
    let mut parts = p.file_stem()?.to_str()?.split('-');
    if parts.next()? != "requirements" {
        return None;
    }
    let name = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(DependedFile::Requirements(name))
}

fn python_command() -> Command {
    let s = env::var("MOLT_BUILD_PYTHON")
        .map(|v| Path::new(&v).to_path_buf())
        .or_else(|_| which::which("py"))
        .or_else(|_| which::which("python3"))
        .or_else(|_| which::which("python"))
        .unwrap_or_else(|_| {
            println!("cargo:warning=Python not found, defaults to \"python\"");
            println!("cargo:warning=Set MOLT_BUILD_PYTHON to override");
            ["python"].iter().collect()
        });
    Command::new(s.to_str().unwrap())
}

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let assets_dir = Path::new(&root).join("vendor");

    for entry in assets_dir.read_dir().expect("cannot read vendor dir") {
        let entry = entry.expect("cannot read vendor dir entry");
        let path = entry.path();
        if let Some(_) = find_depended_file(&path) {
            if let Some(s) = path.to_str() {
                println!("cargo:rerun-if-changed={}", s);
            }
        }
    }

    let s = python_command()
        .arg(assets_dir.to_str().unwrap())
        .status()
        .expect("failed to execute vendor script");
    std::process::exit(s.code().unwrap_or(-1));
}
