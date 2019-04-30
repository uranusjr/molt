use std::path::{Path, PathBuf};

pub enum Foreign {
    PipfileLock(PathBuf),
    PoetryLock(PathBuf),
}

impl Foreign {
    pub fn find_in(path: &Path) -> Option<Self> {
        let mut p: PathBuf;

        p = path.join("Pipfile.lock");
        if p.is_file() {
            return Some(Foreign::PipfileLock(p));
        }

        p = path.join("poetry.lock");
        if p.is_file() {
            return Some(Foreign::PoetryLock(p));
        }

        None
    }
}
