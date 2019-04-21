use std::fs::{File, create_dir_all};
use std::io::{Result, Write};
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
                let mut f = File::create(target)?;
                f.write_all(&data.into_owned())?;
            }
            Ok(())
        }
    };
}

#[derive(RustEmbed)]
#[folder = "assets/virtenv"]
pub struct VirtEnv;

impl VirtEnv {
    pub fn populate_to(dir: &Path) -> Result<()> {
        populate!(Self, dir)
    }
}

#[derive(RustEmbed)]
#[folder = "assets/pep425"]
pub struct Pep425;

impl Pep425 {
    pub fn populate_to(dir: &Path) -> Result<()> {
        populate!(Self, dir)
    }
}
