use std::collections::{HashMap, hash_map};
use std::path::Path;

use ini::Ini;
use regex::Regex;

pub struct EntryPoint {
    modu: String,
    func: String,

    #[allow(dead_code)] gui: bool,
}

impl EntryPoint {
    fn parse(value: &str, gui: bool) -> Option<Self> {
        let (m, f) = value.split_at(value.find(':')?);
        Some(Self {
            modu: m.trim().to_string(),
            func: f[1..].trim().to_string(),
            gui,
        })
    }

    pub fn module(&self) -> &str {
        &self.modu
    }

    pub fn function(&self) -> &str {
        &self.func
    }
}

lazy_static! {
    static ref PIP_RE: Regex =
        Regex::new(r"^pip\d+(\.\d+)?$").unwrap();
    static ref EASY_INSTALL_RE: Regex =
        Regex::new(r"^easy_install\-\d+(\.\d+)?$").unwrap();
}

fn read_entry_points(distro: &Path) -> Option<HashMap<String, EntryPoint>> {
    if !distro.is_dir() {
        return None;
    }
    match distro.extension() {
        None => { return None; },
        Some(e) => if e != "dist-info" && e != "egg-info" { return None; },
    }
    let entry_points_txt = distro.join("entry_points.txt");
    if !entry_points_txt.is_file() {
        return None;
    }

    let mut entry_points = HashMap::new();
    for (section, properties) in &Ini::load_from_file(entry_points_txt).ok()? {
        let gui = match section.as_ref().map(String::as_str) {
            Some("console_scripts") => { false },
            Some("gui_scripts") => { true },
            _ => { continue; },
        };
        for (key, value) in properties.iter() {
            // Blacklist versioned pip and easy_install entries.
            // github.com/pypa/pip/blob/54b6a91/src/pip/_internal/wheel.py#L507
            if PIP_RE.is_match(key) || EASY_INSTALL_RE.is_match(key) {
                continue;
            }
            let entry_point = match EntryPoint::parse(value, gui) {
                Some(v) => v,
                None => { continue; },
            };
            entry_points.insert(key.trim().to_string(), entry_point);
        }
    }
    Some(entry_points)
}

fn read_all_entry_points(dir: &Path) -> Option<HashMap<String, EntryPoint>> {
    let mut entry_points = HashMap::new();
    for read_result in dir.read_dir().ok()? {
        let entry = match read_result {
            Ok(e) => e,
            Err(_) => { continue; },
        };
        match read_entry_points(&entry.path()) {
            Some(h) => { entry_points.extend(h); },
            None => { continue; },
        }
    }
    Some(entry_points)
}

// TODO: Implement this as a lazy iterator instead.
pub struct EntryPoints {
    iterator: hash_map::IntoIter<String, EntryPoint>,
}

impl EntryPoints {
    pub fn new(site_packages: &Path) -> Self {
        let members = read_all_entry_points(site_packages).unwrap_or_default();
        Self { iterator: members.into_iter() }
    }
}

impl Iterator for EntryPoints {
    type Item = (String, EntryPoint);
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}
