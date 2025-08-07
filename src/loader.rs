use crate::error::Result;

pub trait PropertyLoader {
    /// Loads a string value from the configuration backend.
    ///
    /// This method returns `Some(String)` if the key exists and has a string value,
    /// or `None` if the key does not exist.
    #[cfg(feature = "std")]
    fn load_str_value(&self, key: &str) -> Result<String>;

    #[cfg(not(feature = "std"))]
    fn load_str_value<const N: usize>(&self, key: &str) -> Result<heapless::String<N>>;

    /// Loads an integer value from the configuration backend.
    ///
    /// This method returns `Some(i32)` if the key exists and has an integer value,
    /// or `None` if the key does not exist.
    fn load_number_value(&self, key: &str) -> Result<i32>;

    /// Loads a boolean value from the configuration backend.
    ///
    /// This method returns `Some(bool)` if the key exists and has a boolean value,
    /// or `None` if the key does not exist.
    fn load_bool_value(&self, key: &str) -> Result<bool>;
}

