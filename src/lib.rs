//! # figen
//! This crate provides a way to define and load configurations in Rust, allowing for structured configuration management.
//! It supports both standard and no-std environments, making it versatile for various applications.
//!
//! # Example
//! The following example demonstrates how to define a configuration structure using the `Configuration` derive macro and load it using a custom property loader.
//! The `Configuration` derive macro automatically generates the necessary bindings for the configuration fields, allowing you to easily manage and load configurations from various sources.
//! ```ignore
//! use figen::{Configuration, load_config};
//!
//! #[derive(Configuration, Debug, Default)]
//! struct MyConfig {
//!     #[property]
//!     my_field: String,
//! }
//!
//! fn main() {
//!     let loader = MyPropertyLoader::new(); // Implement your own PropertyLoader
//!     let config: MyConfig = load.config(&loader)
//!         .expect("Failed to load configuration");
//!     println!("{:?}", config);
//! }
//! ```
//!
//! A more typical usage would involve using the `config_registry` macro to define a configuration registry, which also generates the necessary bindings and structs for you.
//! ```ignore
//! use figen::config_registry;
//! config_registry!(
//!     name = MyGroupConfig
//!     version = 1
//!     str_property("my_str", default = "default_value", max_len = 10)
//!     num_property("my_nested.prop", default = 42)
//! );
//!
//! fn main() {
//!     let loader = MyPropertyLoader::new(); // Implement your own PropertyLoader
//!     let config: MyGroupConfig = figen::load_config(&loader)
//!        .expect("Failed to load configuration");
//!     println!("{:?}", config);
//! }
//! ```
//!
//! # Using Custom Types
//! You can also define custom types for your configuration properties. The `config_registry` macro allows
//! you to specify a custom type for a property, and it will generate the necessary bindings for
//! that type. The custom type must implement the `TryFrom<&str>` trait to convert the string value
//! into the custom type. Once the type implements `TryFrom<&str>`, derive `ConfigBinder` on the type.
//!
//! ## Example
//! ```ignore
//! use figen::config_registry;
//!
//! #[derive(figen::ConfigBinder)]
//! struct CustomType {
//!    field1: i32,
//!    field2: bool,
//! }
//!
//! impl TryFrom<&str> for CustomType {
//!    type Error = &'static str;
//!
//!    fn try_from(value: &str) -> Result<Self, Self::Error> {
//!      // Custom conversion logic from &str:
//!      // parse `value` and return `Ok(CustomType { ... })` or `Err(...)`
//!      unimplemented!()
//!    }
//! }
//!
//! config_registry!(
//!    name = MyGroupConfig
//!    version = 1
//!    custom_property("my_custom", default = "1,true", ty = CustomType)
//! )
//! ```
//!
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate alloc;

pub mod binder;
pub mod error;
pub mod loader;
pub mod registry;

use binder::ConfigBinder;
use loader::PropertyLoader;

// Re-export
pub use crate::error::Result;
pub use figen_proc_macros::expand_config_registry as config_registry;
pub use figen_proc_macros::ConfigBinder;
pub use figen_proc_macros::Configuration;

pub trait BindPath {
    fn new() -> Self;
    fn push(&mut self, key: &str);
    fn pop(&mut self);
    fn push_array_index(&mut self, key: &str);
    fn pop_array_index(&mut self);
    fn current_path(&self) -> &str;
}

pub struct NoStdBindPath<const N: usize> {
    path_separator: heapless::String<1>,
    path: heapless::String<N>,
}

impl<const N: usize> BindPath for NoStdBindPath<N> {
    fn new() -> Self {
        use core::str::FromStr;

        NoStdBindPath {
            path_separator: heapless::String::from_str(".")
                .expect("Failed to create path separator"),
            path: heapless::String::new(),
        }
    }

    fn push(&mut self, key: &str) {
        if !self.path.is_empty() {
            self.path
                .push_str(self.path_separator.as_str())
                .expect("Failed to push path separator");
        }
        self.path.push_str(key).expect("Failed to push key to path");
    }

    fn pop(&mut self) {
        if let Some(last_separator) = self.path.rfind(self.path_separator.as_str()) {
            self.path.truncate(last_separator);
        } else {
            self.path.clear();
        }
    }

    fn push_array_index(&mut self, key: &str) {
        self.path
            .push('[')
            .expect("Failed to push array index opening bracket");
        self.path
            .push_str(key)
            .expect("Failed to push array index key");
        self.path
            .push(']')
            .expect("Failed to push array index closing bracket");
    }

    fn pop_array_index(&mut self) {
        if let Some(last_bracket) = self.path.rfind('[') {
            self.path.truncate(last_bracket);
        } else {
            self.path.clear();
        }
    }

    fn current_path(&self) -> &str {
        self.path.as_str()
    }
}

#[cfg(feature = "std")]
pub struct StdBindPath {
    path_separator: String,
    path: String,
}

#[cfg(feature = "std")]
impl BindPath for StdBindPath {
    fn new() -> Self {
        Self {
            path_separator: String::from("."),
            path: String::new(),
        }
    }

    fn push(&mut self, key: &str) {
        if !self.path.is_empty() {
            self.path.push_str(self.path_separator.as_str());
        }
        self.path.push_str(key);
    }

    fn pop(&mut self) {
        let last_separator = self.path.rfind(self.path_separator.as_str());
        if let Some(index) = last_separator {
            self.path.truncate(index);
        } else {
            self.path.clear();
        }
    }

    fn push_array_index(&mut self, key: &str) {
        self.path.push('[');
        self.path.push_str(key);
        self.path.push(']');
    }

    fn pop_array_index(&mut self) {
        if let Some(last_bracket) = self.path.rfind('[') {
            self.path.truncate(last_bracket);
        } else {
            self.path.clear();
        }
    }

    fn current_path(&self) -> &str {
        self.path.as_str()
    }
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! str_ty {
    () => {
        std::string::String
    };
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! str_ty {
    () => { heapless::String<32> };
}

#[cfg(not(feature = "std"))]
pub type BindPathImpl = NoStdBindPath<64>;
#[cfg(feature = "std")]
pub type BindPathImpl = StdBindPath;

#[inline(always)]
pub fn load_config<T, U>(property_loader: &U) -> error::Result<T>
where
    T: ConfigBinder<BindPathImpl, U> + Default,
    U: PropertyLoader,
{
    load_config_using_path(property_loader)
}

pub fn load_config_using_path<T, U, P>(property_loader: &U) -> error::Result<T>
where
    T: ConfigBinder<P, U> + Default,
    U: PropertyLoader,
    P: BindPath,
{
    // Create a new path and config instance to bind to
    let mut current_path = P::new();
    let mut config = T::default();

    config.bind(&mut current_path, property_loader)?;
    Ok(config)
}
