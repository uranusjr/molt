use std::collections::HashMap;
use std::fmt::{self, Formatter};

use serde::de::{
    self,
    Deserialize,
    Deserializer,
    MapAccess,
    Visitor,
};

use super::{
    Dependencies,
    DependencyEntry,
    Hashes,
    Sources,
};

pub struct Lock {
    #[allow(dead_code)]
    sources: Sources,

    dependencies: Dependencies,
}

impl<'a> Lock {
    pub fn from(sources: Sources, dependencies: Dependencies) -> Self {
        Self { sources, dependencies }
    }

    #[cfg(test)]
    pub fn sources(&self) -> &Sources {
        &self.sources
    }

    pub fn dependencies(&self) -> &Dependencies {
        &self.dependencies
    }
}

impl<'de> Deserialize<'de> for Lock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field { Sources, Dependencies, Hashes }

        struct LockVisitor;

        impl<'de> Visitor<'de> for LockVisitor {
            type Value = Lock;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("`sources`, `dependencies`, or `hashes`")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where A: MapAccess<'de>
            {
                let mut sources: Option<Sources> = None;
                let mut dents: Option<HashMap<String, DependencyEntry>> = None;
                let mut hashes: Option<HashMap<String, Hashes>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Sources => {
                            if sources.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "sources",
                                ));
                            }
                            sources = Some(map.next_value::<Sources>()?);
                        },
                        Field::Dependencies => {
                            if dents.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "dependencies",
                                ));
                            }
                            dents = Some(map.next_value()?);
                        },
                        Field::Hashes => {
                            if hashes.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "hashes",
                                ));
                            }
                            hashes = Some(map.next_value()?);
                        },
                    }
                }

                let sources = sources.unwrap_or_default();
                let dents = dents.unwrap_or_default();
                let mut hashes = hashes.unwrap_or_default();

                // Convert the dependencies into semi-concrete objects, with
                // hashes injected and sources resolved, but edges are not
                // connected at this point.
                let mut dependencies = Dependencies::new();
                let mut links = vec![];
                for (k, v) in dents.into_iter() {
                    let python = v.make_python(&sources, hashes.remove(&k))?;
                    dependencies.add_dependency(&k, python);
                    links.push((k, v.into_dependencies()));
                }

                // Connect the edges.
                for (p, links) in links.into_iter() {
                    for (c, m) in links.into_iter() {
                        let result = dependencies.add_dependence(&p, &c, m);
                        if let Err(k) = result {
                            return Err(de::Error::custom(format!(
                                "unresolvable dependency name {:?}", k,
                            )));
                        }
                    }
                }

                Ok(Lock { sources, dependencies })
            }
        }
        deserializer.deserialize_map(LockVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use serde_json::from_str;

    #[test]
    fn test_simple_dependency_graph() {
        static JSON: &str = r#"{
            "dependencies": {
                "bar": {
                    "python": {"name": "Bar", "version": "2.2.0"},
                    "dependencies": {"baz": null, "foo": ["os_name == 'nt'"]}
                },
                "baz": {},
                "foo": {}
            }
        }"#;

        let lock: Lock = from_str(JSON).unwrap();
        assert_eq!(
            lock.dependencies().iter().map(|(k, _)| k).collect::<HashSet<_>>(),
            ["foo", "bar", "baz"].iter().cloned().collect());

        let mut deps = lock.dependencies().iter().collect::<Vec<_>>();
        deps.sort_by_key(|(k, _)| k.bytes().collect::<Vec<_>>());

        // 2 entries in `dependencies` don't have a `python` key.
        assert_eq!(deps[1].1.python().is_none(), true);
        assert_eq!(deps[2].1.python().is_none(), true);

        // The `bar` entry.
        assert_eq!(deps[0].1.python().unwrap().name(), "Bar");

        // The `bar` entry has two dependencies, one with markers.
        let bar_deps: HashSet<_> = deps[0].1.dependencies()
            .map(|(d, m)| (d.key().to_string(), m.is_some()))
            .collect();
        assert_eq!(bar_deps, [
            (String::from("baz"), false),
            (String::from("foo"), true),
        ].iter().cloned().collect::<HashSet<_>>());
    }
}
