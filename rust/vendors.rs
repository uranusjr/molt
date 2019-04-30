use std::fs::{create_dir_all, write};
use std::io::Result;
use std::path::Path;

macro_rules! populate {
    ($em:ident, $dir:expr) => {
        {
            for e in $em::iter() {
                let filename = e.into_owned();
                let data = $em::get(&filename)
                    .expect("iter-ed entry should exist");
                let target = $dir.join(&filename);
                if let Some(parent) = target.parent() {
                    create_dir_all(parent)?;
                }
                write(target, data)?;
            }
            Ok(())
        }
    };
}

#[derive(RustEmbed)]
#[folder = "target/assets/molt"]
pub struct Molt;

impl Molt {
    pub fn populate_to(dir: &Path) -> Result<()> {
        populate!(Self, dir)
    }
}

#[derive(RustEmbed)]
#[folder = "target/assets/packaging"]
pub struct Packaging;

impl Packaging {
    pub fn populate_to(dir: &Path) -> Result<()> {
        populate!(Self, dir)
    }
}


#[derive(RustEmbed)]
#[folder = "target/assets/pep425"]
pub struct Pep425;

impl Pep425 {
    pub fn populate_to(dir: &Path) -> Result<()> {
        populate!(Self, dir)
    }
}

#[derive(RustEmbed)]
#[folder = "target/assets/virtenv"]
pub struct VirtEnv;

impl VirtEnv {
    pub fn populate_to(dir: &Path) -> Result<()> {
        populate!(Self, dir)
    }
}
