extern crate figen;

use std::cell::RefCell;
use std::collections::HashMap;

use figen::error::Result;
use figen::loader::PropertyLoader;


#[cfg(not(feature = "std"))]
pub type BindPathImpl = figen::NoStdBindPath<128>;

#[cfg(feature = "std")]
pub type BindPathImpl = figen::StdBindPath;

#[cfg(feature = "std")]
pub type StringType = std::string::String;

#[cfg(not(feature = "std"))]
pub type StringType = heapless::String<16>;

pub struct MockLoader {
    data: HashMap<String, String>,
    attempted_keys: RefCell<Vec<String>>,
}

impl MockLoader {
    pub fn new() -> Self {
        Self { data: HashMap::new(), attempted_keys: RefCell::new(Vec::new()) }
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get_attempted_keys(&self) -> Vec<String> {
        self.attempted_keys.borrow().clone()
    }
}

impl PropertyLoader for MockLoader {
    #[cfg(feature = "std")]
    fn load_str_value(&self, key: &str) -> Result<String> {
        println!("Loading [ str] value for key [{}]", key);
        self.attempted_keys.borrow_mut().push(key.to_string());
        self.data.get(key).cloned().ok_or(figen::error::Error::NotFound)
    }

    #[cfg(not(feature = "std"))]
    fn load_str_value<const N: usize>(&self, key: &str) -> Result<heapless::String<N>> {
        println!("Loading [h str] value for key [{}]", key);
        self.attempted_keys.borrow_mut().push(key.to_string());
        use std::str::FromStr;
        self.data.get(key)
            .and_then(|v| heapless::String::<N>::from_str(v).ok())
            .ok_or(figen::error::Error::NotFound)
    }


    fn load_number_value(&self, key: &str) -> Result<i32> {
        println!("Loading [ i32] value for key [{}]", key);
        self.attempted_keys.borrow_mut().push(key.to_string());
        self.data.get(key)
            .and_then(|v| v.parse::<i32>().ok())
            .ok_or(figen::error::Error::NotFound)
    }

    fn load_bool_value(&self, key: &str) -> Result<bool> {
        println!("Loading [bool] value for key [{}]", key);
        self.attempted_keys.borrow_mut().push(key.to_string());
        self.data.get(key)
            .and_then(|v| v.parse::<bool>().ok())
            .ok_or(figen::error::Error::NotFound)
    }
}