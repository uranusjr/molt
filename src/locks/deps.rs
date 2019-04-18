use std::collections::HashMap;
use std::fmt::{self, Formatter};

use serde::de::{
    Deserialize,
    Deserializer,
    SeqAccess,
    Visitor,
};

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct PythonPackageEntry {
    name: String,
    version: String,
    sources: Option<Vec<String>>,
}

#[derive(Debug)]
struct Marker(Vec<String>);

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
                formatter.write_str("marker strings")
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

#[derive(Debug, Deserialize)]
struct DependencyEntry {
    python: Option<PythonPackageEntry>,

    #[serde(default)]
    dependencies: HashMap<String, Option<Marker>>,
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use serde_json::from_str;
    use super::*;

    impl From<&Marker> for Vec<String> {
        fn from(v: &Marker) -> Self {
            v.0.iter().cloned().collect()
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
            dependencies.get("bar").map(|v| v.is_none()),
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
