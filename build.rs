extern crate walkdir;
extern crate which;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

enum ModuleEntry {
    Directory,
    PythonFile,
    JSONFile,
}

fn find_module_entry(p: &Path) -> Option<ModuleEntry> {
    if p.is_dir() {
        return Some(ModuleEntry::Directory);
    }
    if let Some(s) = p.extension() {
        if s == "py" {
            return Some(ModuleEntry::PythonFile);
        } else if s == "json" {
            return Some(ModuleEntry::JSONFile);
        }
    }
    None
}

enum VendorEntry<'a> {
    Requirements(&'a str),
    Script(&'a str),
}

fn find_vendor_entry(p: &Path) -> Option<VendorEntry> {
    match p.extension()?.to_str()? {
        "py" => Some(VendorEntry::Script(p.file_stem()?.to_str()?)),
        "txt" => Some(VendorEntry::Requirements(p.file_stem()?.to_str()?)),
        _ => None,
    }
}

fn python_command() -> Command {
    let s = env::var("MOLT_BUILD_PYTHON")
        .map(PathBuf::from)
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
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    if let Some(s) = root.join("target").to_str() {
        println!("cargo:rereun-if-changed={}", s);
    }

    for entry in walkdir::WalkDir::new(root.join("python").join("molt")) {
        let entry = entry.expect("cannot read Python source dir entry");
        let path = entry.path();
        if let Some(_) = find_module_entry(&path) {
            if let Some(s) = path.to_str() {
                println!("cargo:rerun-if-changed={}", s);
            }
        }
    }

    let assets_dir = root.join("vendor");
    for entry in assets_dir.read_dir().expect("cannot read vendor dir") {
        let entry = entry.expect("cannot read vendor dir entry");
        let path = entry.path();
        if let Some(_) = find_vendor_entry(&path) {
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
