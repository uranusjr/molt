use std::collections::HashMap;
use std::fmt::{self, Formatter};
use std::rc::Rc;

use serde::de::{
    self,
    Deserialize,
    Deserializer,
    MapAccess,
    Visitor,
};

use super::{
    Dependency,
    DependencyEntry,
    Hashes,
    Sources,
};

#[derive(Default)]
pub struct Dependencies(HashMap<String, Rc<Dependency>>);

#[allow(dead_code)]
pub struct Lock {
    sources: Sources,
    dependencies: Dependencies,
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
                let mut deps: Option<HashMap<String, DependencyEntry>> = None;
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
                            if deps.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "dependencies",
                                ));
                            }
                            deps = Some(map.next_value()?);
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
                let mut hashes = hashes.unwrap_or_default();

                // Convert the dependencies into semi-concrete objects, with
                // hashes injected and sources resolved, but edges are not
                // connected at this point.
                let mut dependencies = HashMap::new();
                let mut links = HashMap::new();
                for (k, v) in deps.unwrap_or_default().into_iter() {
                    let hs = hashes.remove(&k);
                    let dp = v.into_unlinked_dependency(&sources, hs);
                    let (dep, link) = match dp {
                        Ok(d) => d,
                        Err(e) => { return Err(e); },
                    };
                    dependencies.insert(k.to_string(), Rc::new(dep));
                    links.insert(k, link);
                }

                // Connect the edges. I guess the copy is needed because we
                // cannot modify contens in dependencies while referencing it?
                let copied: HashMap<_, _> = dependencies.iter().map(|(k, v)| {
                    (k.to_string(), v.clone())
                }).collect();
                for (k, v) in dependencies.iter_mut() {
                    Rc::get_mut(v).unwrap().populate_dependencies(
                        links.remove(k).unwrap(),
                        &copied,
                    )?;
                }

                Ok(Lock { sources, dependencies: Dependencies(dependencies) })
            }
        }
        deserializer.deserialize_map(LockVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::collections::hash_map;
    use serde_json::from_str;

    impl Dependencies {
        fn keys(&self) -> hash_map::Keys<String, Rc<Dependency>> {
            self.0.keys()
        }
    }

    #[test]
    fn test_simple_dependency_graph() {
        static JSON: &str = r#"{
            "dependencies": {
                "django": {
                    "python": {"name": "Django", "version": "2.2.0"},
                    "dependencies": {"pytz": null, "sqlparse": null}
                },
                "pytz": {},
                "sqlparse": {}
            }
        }"#;

        let lock: Lock = from_str(JSON).unwrap();
        assert_eq!(
            lock.dependencies.keys()
                .map(String::as_str)
                .collect::<HashSet<&str>>(),
            ["django", "pytz", "sqlparse"].iter().cloned().collect());
    }
}
