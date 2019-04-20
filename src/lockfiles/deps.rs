use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{self, Formatter};
use std::rc::Rc;
use std::slice::Iter;

use serde::de::{
    self,
    Deserialize,
    Deserializer,
    SeqAccess,
    Visitor,
};

use super::{Hashes, Source, Sources};

#[derive(Debug)]
pub struct PythonPackage {
    name: String,
    version: String,
    sources: Option<Vec<Rc<Source>>>,
    hashes: Option<Hashes>,
}

impl PythonPackage {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct PythonPackageEntry {
    name: String,
    version: String,
    sources: Option<Vec<String>>,
}

impl PythonPackageEntry {
    fn into_python_package<E>(
        self,
        all_sources: &Sources,
        hashes: Option<Hashes>,
    ) -> Result<PythonPackage, E>
        where E: de::Error
    {
        let sources = match self.sources {
            None => None,
            Some(keys) => {
                let mut objects = vec![];
                for key in keys {
                    if let Some(s) = all_sources.get(&key) {
                        objects.push(s);
                    } else {
                        return Err(de::Error::custom(format!(
                            "unresolvable source name {:?}", key,
                        )));
                    }
                }
                Some(objects)
            },
        };
        Ok(PythonPackage {
            name: self.name,
            version: self.version,
            sources,
            hashes,
        })
    }
}

#[derive(Debug)]
pub struct Marker(Vec<String>);

impl From<Vec<String>> for Marker {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}

impl IntoIterator for Marker {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'de> Deserialize<'de> for Marker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct MarkerVisitor;

        impl<'de> Visitor<'de> for MarkerVisitor {
            type Value = Marker;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("null or marker array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: SeqAccess<'de>
            {
                let mut strings = match seq.size_hint() {
                    Some(h) => Vec::with_capacity(h),
                    None => vec![],
                };
                while let Some(v) = seq.next_element()? {
                    strings.push(v);
                }
                Ok(Marker::from(strings))
            }
        }
        deserializer.deserialize_seq(MarkerVisitor)
    }
}

pub struct Dependencies<'a>(
    Iter<'a, (Rc<RefCell<Dependency>>, Option<Marker>)>,
);

impl<'a> Iterator for Dependencies<'a> {
    type Item = (Ref<'a, Dependency>, Option<&'a Marker>);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(d, m)| ((*d).borrow(), m.as_ref()))
    }
}

#[derive(Debug)]
pub struct Dependency {
    key: String,
    python: Option<PythonPackage>,
    dependencies: Vec<(Rc<RefCell<Dependency>>, Option<Marker>)>,
}

impl Dependency {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn python(&self) -> Option<&PythonPackage> {
        self.python.as_ref()
    }

    pub fn dependencies(&self) -> Dependencies {
        Dependencies(self.dependencies.iter())
    }

    pub(crate) fn populate_dependencies<E>(
        &mut self,
        refs: HashMap<String, Option<Marker>>,
        from: &HashMap<String, Rc<RefCell<Dependency>>>,
    ) -> Result<(), E>
        where E: de::Error
    {
        for (key, marker) in refs.into_iter() {
            match from.get(&key) {
                Some(dep) => {
                    self.dependencies.push((dep.clone(), marker));
                },
                None => {
                    return Err(de::Error::custom(format!(
                        "unresolvable dependency key {:?}", key,
                    )));
                },
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct DependencyEntry {
    python: Option<PythonPackageEntry>,

    #[serde(default)]
    dependencies: HashMap<String, Option<Marker>>,
}

impl DependencyEntry {
    pub(crate) fn into_unlinked_dependency<E>(
        self,
        key: String,
        sources: &Sources,
        hashes: Option<Hashes>,
    ) -> Result<(Dependency, HashMap<String, Option<Marker>>), E>
        where E: de::Error
    {
        let python = match self.python {
            None => None,
            Some(p) => match p.into_python_package(sources, hashes) {
                Ok(p) => Some(p),
                Err(e) => { return Err(e); },
            },
        };
        let dep = Dependency { key, python, dependencies: vec![] };
        Ok((dep, self.dependencies))
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use serde_json::from_str;
    use super::*;

    impl From<&Marker> for Vec<String> {
        fn from(v: &Marker) -> Self {
            v.0.to_vec()
        }
    }

    #[test]
    fn test_python_entry() {
        static JSON: &str = r#"{
            "name": "certifi",
            "version": "2017.7.27.1",
            "sources": ["default"]
        }"#;

        let entry: PythonPackageEntry = from_str(JSON).unwrap();
        assert_eq!(entry, PythonPackageEntry {
            name: String::from("certifi"),
            version: String::from("2017.7.27.1"),
            sources: Some(vec![String::from("default")]),
        });
    }

    #[test]
    fn test_python_entry_missing_sources() {
        static JSON: &str = r#"{
            "name": "certifi",
            "version": "2017.7.27.1"
        }"#;

        let entry: PythonPackageEntry = from_str(JSON).unwrap();
        assert_eq!(entry, PythonPackageEntry {
            name: String::from("certifi"),
            version: String::from("2017.7.27.1"),
            sources: None,
        });
    }

    #[test]
    fn test_dependency_entry() {
        static JSON: &str = r#"{
            "python": {
                "name": "foo",
                "version": "2.18.4"
            },
            "dependencies": {
                "bar": null,
                "baz": ["os_name == 'nt'"],
                "qux": ["os_name != 'nt'", "python_version < '3.5'"]
            }
        }"#;

        let entry: DependencyEntry = from_str(JSON).unwrap();
        let dependencies = &entry.dependencies;

        assert_eq!(
            dependencies.keys().map(String::as_str).collect::<HashSet<&str>>(),
            ["bar", "baz", "qux"].iter().cloned().collect(),
        );
        assert_eq!(
            dependencies.get("bar").map(Option::is_none),
            Some(true),
        );
        assert_eq!(
            dependencies.get("baz").map(|v| v.as_ref().map(|m| m.into())),
            Some(Some(vec![String::from("os_name == 'nt'")])),
        );
        assert_eq!(
            dependencies.get("qux").map(|v| v.as_ref().map(|m| m.into())),
            Some(Some(vec![
                String::from("os_name != 'nt'"),
                String::from("python_version < '3.5'"),
            ])),
        );

        assert_eq!(entry.python, Some(PythonPackageEntry {
            name: String::from("foo"),
            version: String::from("2.18.4"),
            sources: None,
        }));
    }

    #[test]
    fn test_dependency_entry_no_python() {
        static JSON: &str = "{\"dependencies\": {}}";

        let entry: DependencyEntry = from_str(JSON).unwrap();
        assert_eq!(entry.python, None);
    }

    #[test]
    fn test_dependency_entry_no_dependencies() {
        let entry: DependencyEntry = from_str("{}").unwrap();
        assert!(entry.dependencies.is_empty());
    }
}
