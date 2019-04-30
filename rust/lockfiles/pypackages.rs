use std::path::PathBuf;
use std::rc::Rc;

use serde::de;
use url::Url;

use super::{Hashes, Source, Sources};


#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Specifier {
    Version(String, Option<Rc<Source>>),
    Url(url::Url, bool),
    Path(PathBuf),
    Vcs(url::Url, String),
}

#[derive(Clone, Debug)]
pub struct Package {
    name: String,
    specifier: Specifier,
    hashes: Option<Hashes>,
}

impl Package {
    #[cfg(test)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn to_requirement_txt(&self) -> (bool, String) {
        let mut args = vec![];

        match self.specifier {
            Specifier::Version(ref version, ref source) => {
                args.push(format!("{} == {}", self.name, version));
                if let Some(ref source) = source {
                    args.push(format!("--index-url={}", source.base_url()));
                    if source.no_verify_ssl() {
                        if let Some(host) = source.base_url().host_str() {
                            args.push(format!("--trusted-host={}", host));
                        }
                    }
                }
            },
            Specifier::Url(ref url, no_verify_ssl) => {
                let mut url = url.clone();
                url.set_fragment(Some(&format!("egg={}", self.name)));
                args.push(url.to_string());
                if no_verify_ssl {
                    if let Some(host) = url.host_str() {
                        args.push(format!("--trusted-host={}", host));
                    }
                }
            },
            Specifier::Path(ref path) => {
                // TODO: Do a better job handling non-representable paths?
                // E.g. on Windows we can use Win32 API to get a short path.
                args.push(format!("{}", path.to_string_lossy()));
            },
            Specifier::Vcs(ref url, ref rev) => {
                let path = format!("{}@{}", url.path(), rev);

                let mut url = url.clone();
                url.set_path(&path);
                url.set_fragment(Some(&format!("egg={}", self.name)));
                args.push(url.to_string());
            },
        }

        if let Some(ref hashes) = self.hashes {
            for hash in hashes.iter() {
                args.push(String::from("--hash"));
                args.push(format!("{}", hash));
            }
        }

        (self.hashes.is_some(), args.join(" "))
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Entry {
    name: String,

    #[serde(default)] version: Option<String>,
    #[serde(default)] source: Option<String>,

    #[serde(default, with = "url_serde")] url: Option<Url>,
    #[serde(default)] no_verify_ssl: bool,

    #[serde(default)] path: Option<PathBuf>,

    #[serde(default, with = "url_serde")] vcs: Option<Url>,
    #[serde(default)] rev: Option<String>,
}

impl Entry {
    #[cfg(test)]
    pub fn new_versioned(
        name: &str,
        version: &str,
        source: Option<&str>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            version: Some(version.to_owned()),
            source: source.map(String::from),
            url: None,
            no_verify_ssl: false,
            path: None,
            vcs: None,
            rev: None,
        }
    }

    pub(super) fn make_python_package<E>(
        &self,
        sources: &Sources,
        hashes: Option<Hashes>,
    ) -> Result<Package, E>
        where E: de::Error
    {
        let mut specifier = None;

        if let Some(ref v) = self.version {
            let source = match self.source {
                Some(ref k) => match sources.get(&k) {
                    Some(s) => Some(s),
                    None => {
                        let s = format!("unresolvable source {:?}", k);
                        return Err(de::Error::custom(s));
                    },
                },
                None => None,
            };
            specifier = Some(Specifier::Version(v.to_owned(), source));
        }
        if let Some(ref url) = self.url {
            if specifier.is_some() {
                return Err(de::Error::custom("redundant package fields"));
            }
            specifier = Some(Specifier::Url(
                url.to_owned(), self.no_verify_ssl,
            ));
        }
        if let Some(ref path) = self.path {
            if specifier.is_some() {
                return Err(de::Error::custom("redundant package fields"));
            }
            specifier = Some(Specifier::Path(path.to_owned()));
        }
        if let Some(ref vcs) = self.vcs {
            if specifier.is_some() {
                return Err(de::Error::custom("redundant package fields"));
            }
            let rev = match self.rev {
                Some(ref r) => r.to_owned(),
                None => { return Err(de::Error::missing_field("rev")); },
            };
            specifier = Some(Specifier::Vcs(vcs.to_owned(), rev));
        }

        match specifier {
            Some(specifier) => Ok(Package {
                name: self.name.clone(),
                specifier,
                hashes,
            }),
            None => Err(de::Error::custom("missing package fields")),
        }
    }
}
