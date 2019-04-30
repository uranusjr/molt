use std::path::PathBuf;

pub enum Foreign {
    PipfileLock(PathBuf),
    PoetryLock(PathBuf),
}
