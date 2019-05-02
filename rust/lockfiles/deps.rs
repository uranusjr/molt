use std::cell::{Ref, RefCell};
use std::collections::{HashMap, hash_map};
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

use super::{Hashes, PythonPackage, Sources};
use super::pypackages::{Entry as PythonPackageEntry};


#[derive(Clone, Debug)]
pub struct Marker(Vec<String>);

impl Marker {
    pub fn iter(&self) -> Iter<String> {
        self.0.iter()
    }
}

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

type DependencyCell = Rc<RefCell<Dependency>>;

type DependencyRef<'a> = Ref<'a, Dependency>;

pub struct IterPackageDependency<'a>(
    Iter<'a, (DependencyCell, Option<Marker>)>,
);

impl<'a> Iterator for IterPackageDependency<'a> {
    type Item = (DependencyRef<'a>, Option<&'a Marker>);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(d, m)| (d.borrow(), m.as_ref()))
    }
}

#[derive(Debug)]
pub struct Dependency {
    key: String,
    python: Option<PythonPackage>,
    dependencies: Vec<(DependencyCell, Option<Marker>)>,
}

impl Dependency {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn python(&self) -> Option<&PythonPackage> {
        self.python.as_ref()
    }

    pub fn dependencies(&self) -> IterPackageDependency {
        IterPackageDependency(self.dependencies.iter())
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct DependencyEntry {
    python: Option<PythonPackageEntry>,

    #[serde(default)]
    dependencies: HashMap<String, Option<Marker>>,
}

impl DependencyEntry {
    pub fn swap_out_python<E>(
        &mut self,
        sources: &Sources,
        hashes: Option<Hashes>,
    ) -> Result<Option<PythonPackage>, E>
        where E: de::Error
    {
        self.python.take().map(|p| {
            p.into_python_package(sources, hashes)
        }).transpose()
    }

    pub fn into_dependencies(self) -> HashMap<String, Option<Marker>> {
        self.dependencies
    }
}

pub struct IterDependency<'a>(hash_map::Iter<'a, String, DependencyCell>);

impl<'a> Iterator for IterDependency<'a> {
    type Item = (&'a str, DependencyRef<'a>);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_str(), v.borrow()))
    }
}

#[derive(Default)]
pub struct Dependencies(HashMap<String, DependencyCell>);

impl Dependencies {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn default(&self) -> Option<Ref<Dependency>> {
        self.0.get("").map(|r| r.borrow())
    }

    pub fn extra(&self, extra: &str) -> Option<Ref<Dependency>> {
        self.0.get(&format!("[{}]", extra)).map(|r| r.borrow())
    }

    #[allow(dead_code)]
    pub fn iter(&self) -> IterDependency {
        IterDependency(self.0.iter())
    }

    pub fn add_dependency(
        &mut self,
        key: &str,
        python: Option<PythonPackage>,
    ) -> Option<DependencyCell> {
        let dep = Dependency {
            key: key.to_string(),
            python,
            dependencies: vec![],
        };
        self.0.insert(key.to_string(), Rc::new(RefCell::new(dep)))
    }

    pub fn add_dependence(
        &mut self,
        dependent: &str,
        depended: &str,
        marker: Option<Marker>,
    ) -> Result<(), String> {
        let depended = self.0.get(depended)
            .ok_or_else(|| depended.to_string())?
            .clone();
        let mut dependent = self.0.get(dependent)
            .ok_or_else(|| dependent.to_string())?
            .borrow_mut();  // TODO: Return an error if this borrow fails?
        dependent.dependencies.push((depended, marker));
        Ok(())
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
    fn test_python_entry_missing_sources() {
        static JSON: &str = r#"{
            "name": "certifi",
            "version": "2017.7.27.1"
        }"#;

        let entry: PythonPackageEntry = from_str(JSON).unwrap();
        assert_eq!(entry, PythonPackageEntry::new_versioned(
                "certifi", "2017.7.27.1", None,
        ));
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

        assert_eq!(
            entry.python,
            Some(PythonPackageEntry::new_versioned("foo", "2.18.4", None)),
        );
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
