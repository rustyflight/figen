#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod loader;
pub mod binder;
pub mod error;
pub mod registry;

use binder::ConfigBinder;
use loader::PropertyLoader;

// Re-export
pub use figen_proc_macros::Configuration;
pub use figen_proc_macros::expand_config_registry;

pub trait BindPath {

    fn new() -> Self;
    fn push(&mut self, key: &str);
    fn pop(&mut self);
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
            path_separator: heapless::String::from_str(".").expect("Failed to create path separator"),
            path: heapless::String::new(),
        }
    }

    fn push(&mut self, key: &str) {
        if !self.path.is_empty() {
            self.path.push_str(self.path_separator.as_str()).expect("Failed to push path separator");
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

    fn current_path(&self) -> &str {
        self.path.as_str()
    }
}

pub fn load_config<T: ConfigBinder<P, U> + Default, U: PropertyLoader, P: BindPath>(property_loader: &U) -> error::Result<T> {
    let mut current_path = P::new();
    let mut config = T::default();

    config.bind(&mut current_path, property_loader)?;
    Ok(config)
}
