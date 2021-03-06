use std::collections::{HashSet, hash_set};
use std::fmt::{self, Formatter};

use serde::de::{
    self,
    Deserialize,
    Deserializer,
    SeqAccess,
    Unexpected,
    Visitor,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Hash {
    name: String,
    value: String,
}

impl Hash {
    fn new(name: &str, value: &str) -> Self {
        Self { name: name.to_string(), value: value.to_string() }
    }

    pub fn parse(v: &str) -> Option<Self> {
        let mut it = v.split(':');
        Some(Hash::new(it.next()?, it.next()?))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.value)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct HashVisitor;

        impl<'de> Visitor<'de> for HashVisitor {
            type Value = Hash;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("hash")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where E: de::Error
            {
                Hash::parse(v).ok_or_else(|| {
                    de::Error::invalid_value(
                        Unexpected::Str(v), &"<name>:<value>",
                    )
                })
            }
        }
        deserializer.deserialize_str(HashVisitor)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Hashes(HashSet<Hash>);

impl Hashes {
    pub fn iter(&self) -> hash_set::Iter<Hash> {
        self.0.iter()
    }
}

impl<'de> Deserialize<'de> for Hashes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct HashesVisitor;

        impl<'de> Visitor<'de> for HashesVisitor {
            type Value = Hashes;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("hash array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: SeqAccess<'de>
            {
                let mut hashes = match seq.size_hint() {
                    Some(h) => HashSet::with_capacity(h),
                    None => HashSet::new(),
                };
                while let Some(v) = seq.next_element()? {
                    hashes.insert(v);
                }
                Ok(Hashes(hashes))
            }
        }
        deserializer.deserialize_seq(HashesVisitor)
    }
}


#[cfg(test)]
mod tests {
    use serde_json::from_str;
    use super::*;

    #[test]
    fn test_hash_deserialize() {
        static N: &str = "sha256";
        static V: &str = "54a07c09c586b0e4c619f02a5e94e36619da8e2b053e20f5943";

        let hash: Hash = from_str(&format!("\"{}:{}\"", N, V)).unwrap();
        assert_eq!(hash, Hash::new(N, V));
    }

    #[test]
    fn test_hashes_deserialize() {
        static JSON: &str = r#"[
            "sha256:54a07c09c586b0e4c619f02a5e94e36619da8e2b053e20f594348c",
            "sha256:40523d2efb60523e113b44602298f0960e900388cf3bb6043f645c"
        ]"#;

        let hashes: Hashes = from_str(JSON).unwrap();
        assert_eq!(hashes.0.len(), 2);
        assert!(hashes.0.contains(&Hash::new(
            "sha256", "54a07c09c586b0e4c619f02a5e94e36619da8e2b053e20f594348c",
        )));
        assert!(hashes.0.contains(&Hash::new(
            "sha256", "40523d2efb60523e113b44602298f0960e900388cf3bb6043f645c",
        )));
    }
}
