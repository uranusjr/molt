use std::cell::{Ref, RefCell};
use std::collections::{HashMap, hash_map};
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
pub struct Dependencies(HashMap<String, DependencyRef>);

type DependencyRef = Rc<RefCell<Dependency>>;

pub struct IterDependency<'a>(hash_map::Iter<'a, String, DependencyRef>);

impl<'a> Iterator for IterDependency<'a> {
    type Item = (&'a str, Ref<'a, Dependency>);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_str(), (*v).borrow()))
    }
}

#[allow(dead_code)]
pub struct Lock {
    sources: Sources,
    dependencies: Dependencies,
}

impl<'a> Lock {
    fn dependencies(&self) -> IterDependency {
        IterDependency(self.dependencies.0.iter())
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
                let mut hashes = hashes.unwrap_or_default();

                // Convert the dependencies into semi-concrete objects, with
                // hashes injected and sources resolved, but edges are not
                // connected at this point.
                let mut deps = HashMap::new();
                let mut links = HashMap::new();
                for (k, v) in dents.unwrap_or_default().into_iter() {
                    let hash = hashes.remove(&k);
                    let dp = v.into_unlinked_dependency(
                        k.to_string(), &sources, hash,
                    );
                    let (dep, link) = match dp {
                        Ok(d) => d,
                        Err(e) => { return Err(e); },
                    };
                    deps.insert(
                        k.to_string(),
                        Rc::new(RefCell::new(dep)));
                    links.insert(k, link);
                }

                // Connect the edges.
                for (k, v) in deps.iter().map(|(k, v)| (k, v.clone())) {
                    v.borrow_mut().populate_dependencies(
                        links.remove(k).unwrap(),
                        &deps,
                    )?;
                }

                let dependencies = Dependencies(deps);
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
            lock.dependencies().map(|(k, _)| k).collect::<HashSet<_>>(),
            ["foo", "bar", "baz"].iter().cloned().collect());

        let mut deps = lock.dependencies().collect::<Vec<_>>();
        deps.sort_by_key(|(k, _)| k.bytes().next());

        // 2 entries in `dependencies` don't have a `python` key.
        assert_eq!((*deps[1].1).python().is_none(), true);
        assert_eq!((*deps[2].1).python().is_none(), true);

        // The `bar` entry.
        assert_eq!((*deps[0].1).python().unwrap().name(), "Bar");

        // The `bar` entry has two dependencies, one with markers.
        let bar_deps: HashSet<_> = (*deps[0].1).dependencies()
            .map(|(d, m)| (d.key().to_string(), m.is_some()))
            .collect();
        assert_eq!(bar_deps, [
            (String::from("baz"), false),
            (String::from("foo"), true),
        ].iter().cloned().collect::<HashSet<_>>());
    }
}
