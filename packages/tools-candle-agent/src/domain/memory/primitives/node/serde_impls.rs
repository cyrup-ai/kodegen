use std::sync::Arc;

use crossbeam_skiplist::SkipMap;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::SerializeStruct,
};

use super::{MemoryNode, MemoryNodeMetadata, MemoryNodeStats};

impl Serialize for MemoryNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("MemoryNode", 2)?;
        state.serialize_field("base_memory", &self.base_memory)?;
        state.serialize_field("embedding", &self.embedding)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for MemoryNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            BaseMemory,
            Embedding,
        }

        struct MemoryNodeVisitor;

        impl<'de> Visitor<'de> for MemoryNodeVisitor {
            type Value = MemoryNode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct MemoryNode")
            }

            fn visit_map<V>(self, mut map: V) -> Result<MemoryNode, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut base_memory = None;
                let mut embedding = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::BaseMemory => {
                            if base_memory.is_some() {
                                return Err(serde::de::Error::duplicate_field("base_memory"));
                            }
                            base_memory = Some(map.next_value()?);
                        }
                        Field::Embedding => {
                            if embedding.is_some() {
                                return Err(serde::de::Error::duplicate_field("embedding"));
                            }
                            embedding = Some(map.next_value()?);
                        }
                    }
                }

                let base_memory =
                    base_memory.ok_or_else(|| serde::de::Error::missing_field("base_memory"))?;
                let embedding = embedding.unwrap_or(None);

                Ok(MemoryNode {
                    base_memory,
                    embedding,
                    metadata: Arc::new(MemoryNodeMetadata::new()),
                    relationships: Arc::new(SkipMap::new()),
                    stats: Arc::new(MemoryNodeStats::new()),
                })
            }
        }

        const FIELDS: &[&str] = &["base_memory", "embedding"];
        deserializer.deserialize_struct("MemoryNode", FIELDS, MemoryNodeVisitor)
    }
}
