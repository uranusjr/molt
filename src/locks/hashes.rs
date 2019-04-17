use std::collections::HashSet;
use std::fmt::{self, Formatter};

use serde::de::{
    self,
    Deserialize,
    Deserializer,
    SeqAccess,
    Unexpected,
    Visitor,
};

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Hash {
    name: String,
    value: String,
}

impl Hash {
    fn new(name: &str, value: &str) -> Self {
        Self { name: name.to_string(), value: value.to_string() }
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
                formatter.write_str("struct Hash")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where E: de::Error
            {
                let mut it = v.split(':');
                let name = it.next().unwrap();
                match it.next() {
                    Some(v) => Ok(Hash::new(name, v)),
                    None => Err(de::Error::invalid_value(
                        Unexpected::Str(v), &"<name>:<value>",
                    )),
                }
            }
        }
        deserializer.deserialize_str(HashVisitor)
    }
}

#[derive(Debug)]
pub struct Hashes {
    hashes: HashSet<Hash>,
}

impl Hashes {
    fn new(hashes: HashSet<Hash>) -> Self {
        Self { hashes }
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn contains(&self, value: &Hash) -> bool {
        self.hashes.contains(value)
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
                formatter.write_str("struct Hashes")
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
                Ok(Hashes::new(hashes))
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
        let json = r#"[
            "sha256:54a07c09c586b0e4c619f02a5e94e36619da8e2b053e20f594348c",
            "sha256:40523d2efb60523e113b44602298f0960e900388cf3bb6043f645c"
        ]"#;

        let hashes: Hashes = from_str(json).unwrap();
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains(&Hash::new(
            "sha256", "54a07c09c586b0e4c619f02a5e94e36619da8e2b053e20f594348c",
        )));
        assert!(hashes.contains(&Hash::new(
            "sha256", "40523d2efb60523e113b44602298f0960e900388cf3bb6043f645c",
        )));
    }
}
