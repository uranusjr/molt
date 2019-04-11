use std::fs::{File, create_dir_all};
use std::io::{Result, Write};
use std::path::Path;

#[derive(RustEmbed)]
#[folder = "assets/virtenv"]
pub struct VirtEnv;

impl VirtEnv {
    pub fn populate_to(dir: &Path) -> Result<()> {
        for e in Self::iter() {
            let filename = e.into_owned();
            let data = Self::get(&filename)
                .expect("iter-ed entry should exist");
            let target = dir.join(&filename);
            if let Some(parent) = target.parent() {
                create_dir_all(parent)?;
            }
            let mut f = File::create(target)?;
            f.write_all(&data.into_owned())?;
        }
        Ok(())
    }
}
