use std::collections::HashMap;

use crate::lockfiles::{
    Dependencies,
    Hashes,
    Lock,
    PythonPackage,
    PythonPackageSpecifier,
    Sources,
};

#[derive(Debug, Deserialize)]
struct SourceEntry {
    name: String,

    #[serde(default)]
    verify_ssl: bool,

    #[serde(with = "url_serde")]
    url: url::Url,
}

#[derive(Debug, Default, Deserialize)]
struct Meta {
    sources: Vec<SourceEntry>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Clone, Eq, PartialEq))]
struct Package {
    hashes: Option<Hashes>,
    version: String,    // TODO: Support URL and path requirements.

    #[serde(rename = "index")]
    source: Option<String>,

    #[serde(default)]
    editable: bool,
}

fn convert_sources(entries: Vec<SourceEntry>) -> Sources {
    let mut sources = Sources::new();
    for entry in entries {
        sources.add(entry.name, entry.url, !entry.verify_ssl);
    }
    sources
}

fn convert_python_packages(
    entries: HashMap<String, Package>,
    sources: &Sources,
) -> Vec<(String, PythonPackage)> {
    let mut packages = vec![];
    for (name, entry) in entries {
        // TODO: Post warning if editable.
        let spec = PythonPackageSpecifier::Version(
            entry.version[2..].to_string(),
        );
        let source = entry.source.and_then(|s| sources.get(&s));
        let hashes = entry.hashes;
        let mut package = PythonPackage::new(&name, spec, source, hashes);
        packages.push((name, package));
    }
    packages
}

#[derive(Debug, Deserialize)]
pub struct PipfileLock {
    default: HashMap<String, Package>,
    develop: HashMap<String, Package>,

    #[serde(default)]
    #[serde(rename = "_meta")]
    meta: Meta,
}

impl PipfileLock {
    #[allow(dead_code)]
    pub fn into_lock(self) -> Lock {
        let sources = convert_sources(self.meta.sources);

        let default = convert_python_packages(self.default, &sources);
        let develop = convert_python_packages(self.develop, &sources);

        let mut dependencies = Dependencies::new();

        // Sections.
        dependencies.add_dependency("", None);
        dependencies.add_dependency("[dev]", None);

        // Section aliases.
        dependencies.add_dependency("[default]", None);
        dependencies.add_dependence("[default]", "", None).unwrap();
        dependencies.add_dependency("[develop]", None);
        dependencies.add_dependence("[develop]", "[dev]", None).unwrap();

        for (s, p) in default {
            dependencies.add_dependency(&s, Some(p));
            dependencies.add_dependence("", &s, None).unwrap();
        }
        for (s, p) in develop {
            dependencies.add_dependency(&s, Some(p));
            dependencies.add_dependence("[dev]", &s, None).unwrap();
        }

        Lock::from(sources, dependencies)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use serde_json::from_str;
    use crate::lockfiles::Hash;

    impl Package {
        fn new<'a, I>(version: &str, hash_values: I) -> Self
            where I: Iterator<Item=&'a str>
        {
            let mut hashes = Hashes::new();
            for s in hash_values {
                hashes.add(Hash::parse(s).unwrap());
            }
            Self {
                version: version.to_string(),
                hashes: Some(hashes),
                source: None,
                editable: false,
            }
        }
    }

    #[test]
    fn test_load() {
        static JSON: &str = r#"{
            "default": {
                "certifi": {
                    "hashes": [
                        "sha256:59b7658e26ca9c7339e00f8f4636cdfe59d34fa37b9b0",
                        "sha256:b26104d6835d1f5e49452a26eb2ff87fe7090b89dfcae"
                    ],
                    "version": "==2019.3.9"
                },
                "chardet": {
                    "hashes": [
                        "sha256:84ab92ed1c4d4f16916e05906b6b75a6c0fb5db821cc6",
                        "sha256:fc323ffcaeaed0e0a02bf4d117757b98aed530d9ed453"
                    ],
                    "version": "==3.0.4"
                }
            },
            "develop": {
                "six": {
                    "hashes": [
                        "sha256:3350809f0555b11f552448330d0b52d5f24c91a322ea4",
                        "sha256:d16a0141ec1a18405cd4ce8b4613101da75da0e9a7aec"
                    ],
                    "version": "==1.12.0"
                }
            }
        }"#;
        let pipfile_lock: PipfileLock = from_str(JSON).unwrap();

        let certifi = Package::new(
            "==2019.3.9",
            [
                "sha256:59b7658e26ca9c7339e00f8f4636cdfe59d34fa37b9b0",
                "sha256:b26104d6835d1f5e49452a26eb2ff87fe7090b89dfcae",
            ].iter().cloned(),
        );
        let chardet = Package::new(
            "==3.0.4",
            [
                "sha256:84ab92ed1c4d4f16916e05906b6b75a6c0fb5db821cc6",
                "sha256:fc323ffcaeaed0e0a02bf4d117757b98aed530d9ed453",
            ].iter().cloned(),
        );
        let six = Package::new(
            "==1.12.0",
            [
                "sha256:3350809f0555b11f552448330d0b52d5f24c91a322ea4",
                "sha256:d16a0141ec1a18405cd4ce8b4613101da75da0e9a7aec",
            ].iter().cloned(),
        );

        assert_eq!(pipfile_lock.default, [
            (String::from("certifi"), certifi),
            (String::from("chardet"), chardet),
        ].iter().cloned().collect());

        assert_eq!(pipfile_lock.develop, [
            (String::from("six"), six),
        ].iter().cloned().collect());

        let lock = pipfile_lock.into_lock();

        assert_eq!(
            lock.dependencies().iter()
                .map(|(k, _)| k.to_string())
                .collect::<HashSet<_>>(),
            [
                "", "[dev]", "[default]", "[develop]",
                "certifi", "chardet", "six",
            ].iter().cloned().map(String::from).collect(),
        );

        assert!(lock.dependencies().default().unwrap().python().is_none());
        assert_eq!(
            lock.dependencies().default().unwrap().dependencies()
                .map(|(d, v)| (d.key().to_string(), v.is_some()))
                .collect::<HashMap<_, _>>(),
            [
                (String::from("certifi"), false),
                (String::from("chardet"), false),
            ].iter().cloned().collect(),
        );

        assert_eq!(lock.sources().len(), 0);

        // TODO: Test package contents.
    }
}
