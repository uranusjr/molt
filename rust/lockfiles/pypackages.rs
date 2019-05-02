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
#[serde(untagged)]
enum EntrySpecifier {
    Version { version: String, source: Option<String> },
    Url {
        #[serde(with = "url_serde")] url: Url,
        #[serde(rename = "no_verify_ssl")] trust: bool,
    },
    Path { path: PathBuf },
    Vcs { #[serde(with = "url_serde")] vcs: Url, rev: String },
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Entry {
    name: String,
    #[serde(flatten)] spec: EntrySpecifier,
}

impl Entry {
    pub(super) fn into_python_package<E>(
        self,
        sources: &Sources,
        hashes: Option<Hashes>,
    ) -> Result<Package, E>
        where E: de::Error
    {
        let specifier = match self.spec {
            EntrySpecifier::Version { version: v, source: s} => {
                let source = s.map(|ref k| sources.get(k).ok_or_else(|| {
                    de::Error::custom(format!("unresolvable source {:?}", k))
                })).transpose()?;
                Specifier::Version(v, source)
            },
            EntrySpecifier::Url { url, trust } => Specifier::Url(url, trust),
            EntrySpecifier::Path { path } => Specifier::Path(path),
            EntrySpecifier::Vcs { vcs, rev } => Specifier::Vcs(vcs, rev),
        };
        Ok(Package { name: self.name, specifier, hashes })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;
    use super::*;

    impl Entry {
        pub fn new_versioned(
            name: &str,
            version: &str,
            source: Option<&str>,
        ) -> Self {
            Self {
                name: name.to_owned(),
                spec: EntrySpecifier::Version {
                    version: version.to_owned(),
                    source: source.map(String::from),
                },
            }
        }
    }

    #[test]
    fn test_entry() {
        static JSON: &str = r#"{
            "name": "certifi",
            "version": "2017.7.27.1",
            "source": "default"
        }"#;

        let entry: Entry = from_str(JSON).unwrap();
        assert_eq!(entry, Entry::new_versioned(
            "certifi", "2017.7.27.1", Some("default"),
        ));
    }
}
