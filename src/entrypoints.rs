use std::fs::{ReadDir, read_dir};
use std::path::Path;

use ini::Ini;

pub struct EntryPoint {
    modu: String,
    func: String,
    gui: bool,
}

pub struct EntryPoints {
    readdir: Option<ReadDir>,
}

impl EntryPoints {
    pub fn new(root: &Path) -> Self {
        Self { readdir: read_dir(root).ok() }
    }
}

impl Iterator for EntryPoints {
    type Item = EntryPoint;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let info_path = self.readdir.as_mut()?.next()?.ok()?.path();
            if !info_path.is_dir() {
                continue;
            }
            match info_path.extension() {
                None => { continue; },
                Some(ext) => if ext != "dist-info" { continue; },
            }
            let file_path = info_path.join("entry_points.txt");
            if !file_path.is_file() {
                continue;
            }
            let ini = Ini::load_from_file(file_path).ok()?;
        }
    }
}
