use serde::{Serialize, Serializer};

#[cfg(feature = "std")]
mod std {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct ConfigRegistry<E> {
        pub version: u32,
        pub entries: Vec<RegistryEntry<E>>,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct RegistryEntry<E> {
        pub key: String,
        pub default_value: Option<Value>,
        pub entry_type: E,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Eq, PartialEq, Debug)]
    pub enum Value {
        String(String),
        Number(i32),
        Boolean(bool),
    }
}

#[cfg(not(feature = "std"))]
mod nostd {
    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    pub struct ConfigRegistry<E: 'static> {
        pub version: u32,
        pub entries: &'static [RegistryEntry<E>],
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct RegistryEntry<E> {
        pub key: &'static str,
        pub default_value: Option<Value>,
        pub entry_type: E,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Eq, PartialEq, Debug)]
    pub enum Value {
        String(&'static str),
        Number(i32),
        Boolean(bool),
    }
}

#[cfg(not(feature = "std"))]
pub use nostd::*;

#[cfg(feature = "std")]
pub use std::*;



impl<E: Serialize> ConfigRegistry<E> {
    #[cfg(not(feature = "std"))]
    pub const fn new(version: u32, entries: &'static [RegistryEntry<E>]) -> Self {
        Self {
            version,
            entries
        }
    }

    pub fn get_version(&self) -> u32 {
        self.version
    }

    pub fn has_entry(&self, key: &str) -> bool {
        self.entries.iter().any(|entry| entry.key == key)
    }

    pub fn get_entry(&self, key: &str) -> Option<&RegistryEntry<E>> {
        self.entries.iter().find(|entry| entry.key == key)
    }

    pub fn get_default_value(&self, key: &str) -> Option<&Value> {
        let entry = self.get_entry(key);
        if let Some(entry) = entry {
            entry.default_value.as_ref()
        } else {
            None
        }
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = &RegistryEntry<E>> {
        self.entries.iter()
    }
}

impl<E> RegistryEntry<E> {
    #[cfg(not(feature = "std"))]
    pub const fn new(key: &'static str, entry_type: E, default_value: Option<Value>) -> Self {
        Self {
            key,
            default_value,
            entry_type,
        }
    }
}
