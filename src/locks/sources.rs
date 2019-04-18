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

#[derive(Debug, Eq, PartialEq)]
pub struct Source {
    name: String,
    base_url: String,
    verify_ssl: bool,
}

struct SourceEntry(String, bool);

impl SourceEntry {
    fn into_source(self, name: String) -> Source {
        Source { name, base_url: self.0, verify_ssl: self.1 }
    }
}

impl<'de> Deserialize<'de> for SourceEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field { Url, NoVerifySsl }

        struct SourceEntryVisitor;

        impl<'de> Visitor<'de> for SourceEntryVisitor {
            type Value = SourceEntry;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("`url` or `no_verify_ssl`")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where A: MapAccess<'de>
            {
                let mut url: Option<String> = None;
                let mut ssl: Option<bool> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Url => {
                            if url.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            url = Some(map.next_value()?);
                        },
                        Field::NoVerifySsl => {
                            if ssl.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "no_verify_ssl",
                                ));
                            }
                            ssl = Some(!map.next_value()?);
                        },
                    }
                }

                let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
                let ssl = ssl.unwrap_or(true);
                Ok(SourceEntry(url, ssl))
            }
        }
        deserializer.deserialize_map(SourceEntryVisitor)
    }
}

#[derive(Default)]
pub struct Sources(HashMap<String, Rc<Source>>);

impl Sources {
    pub fn get(&self, key: &str) -> Option<Rc<Source>> {
        self.0.get(key).map(|r| r.clone())
    }
}

impl<'de> Deserialize<'de> for Sources {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct SourcesVisitor;

        impl<'de> Visitor<'de> for SourcesVisitor {
            type Value = Sources;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("source entries")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where A: MapAccess<'de>
            {
                let mut sources = match map.size_hint() {
                    Some(h) => HashMap::with_capacity(h),
                    None => HashMap::new(),
                };
                while let Some(k) = map.next_key()? {
                    let k: &str = k;
                    let v: SourceEntry = map.next_value()?;
                    let source = v.into_source(k.to_string());
                    sources.insert(k.to_string(), Rc::new(source));
                }
                Ok(Sources(sources))
            }
        }
        deserializer.deserialize_map(SourcesVisitor)
    }
}


#[cfg(test)]
mod tests {
    use serde_json::from_str;
    use super::*;

    impl Source {
        fn new(name: &str, base_url: &str, verify_ssl: bool) -> Self {
            Self {
                name: name.to_string(),
                base_url: base_url.to_string(),
                verify_ssl
            }
        }
    }

    #[test]
    fn test_source_mapping() {
        static JSON: &str = r#"{
            "pypi": {"url": "https://pypi.org/simple"},
            "alibaba": {
                "url": "https://mirrors.aliyun.com/simple",
                "no_verify_ssl": true
            }
        }"#;

        let sources: Sources = from_str(JSON).unwrap();
        assert_eq!(sources.0.len(), 2);
        assert_eq!(
            **sources.0.get("pypi").unwrap(),
            Source::new("pypi", "https://pypi.org/simple", true),
        );
        assert_eq!(
            **sources.0.get("alibaba").unwrap(),
            Source::new("alibaba", "https://mirrors.aliyun.com/simple", false),
        );
    }
}
