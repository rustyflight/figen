use crate::error::Error::NotFound;
use crate::loader::PropertyLoader;
use crate::BindPath;
use core::mem::MaybeUninit;

pub const MAX_ARRAY_SIZE: usize = 1024;
const MAX_ARRAY_KEY_SIZE_STR: usize = 4;

pub struct BindContext<T, U> {
    pub path: T,
    pub loader: U,
}

impl<T, U> BindContext<T, U>
where
    T: BindPath,
    U: PropertyLoader,
{
    pub fn new(path: T, loader: U) -> Self {
        Self { path, loader }
    }
}

pub trait ConfigBinder<T, U>
where
    Self: Sized,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()>;
}

/// ConfigBinder implementation for [std::string::String] types.
#[cfg(feature = "std")]
impl<T, U> ConfigBinder<T, U> for std::string::String
where
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let key = path.current_path();
        *self = loader.load_str_value(key)?.into();
        Ok(())
    }
}

/// ConfigBinder implementation for heapless::String.
/// This implementation allows for binding a `heapless::String` to a configuration backend,
/// loading a string value from the backend and storing it in the `heapless::String`.
/// It is designed to work in environments without the standard library, such as embedded systems.
#[cfg(not(feature = "std"))]
impl<const N: usize, T, U> ConfigBinder<T, U> for heapless::String<N>
where
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let key = path.current_path();
        *self = loader.load_str_value(key)?.into();
        Ok(())
    }
}

pub enum ArrayConfigIndicesMode<'a> {
    ZeroIndexed,
    Custom(&'a [&'a str]),
}

/// Binder for arrays in configuration.
/// This binder allows for binding to an array of items in the configuration backend using either zero-based indexing or custom indices.
/// It supports binding to a fixed-size array of items, where each item can be of any type that implements the `ConfigBinder` trait.
pub struct ArrayConfigBinder<'a, T> {
    mode: ArrayConfigIndicesMode<'a>,
    items: &'a mut [T],
}

impl<'a, T> ArrayConfigBinder<'a, T> {
    pub fn new(mode: ArrayConfigIndicesMode<'a>, items: &'a mut [T]) -> Self {
        Self { mode, items }
    }
}

impl<'a, B, U, T> ConfigBinder<T, U> for ArrayConfigBinder<'a, B>
where
    B: ConfigBinder<T, U>,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        use ArrayConfigIndicesMode::*;
        assert!(
            self.items.len() <= MAX_ARRAY_SIZE,
            "Array size exceeds maximum allowed size of {}",
            MAX_ARRAY_SIZE
        );

        let mut result = Err(NotFound);
        match self.mode {
            ZeroIndexed => {
                for (i, item) in self.items.iter_mut().enumerate() {
                    let key: heapless::String<{ MAX_ARRAY_KEY_SIZE_STR }> =
                        heapless::String::try_from(i as u32)
                            .expect("Index too large for heapless::String<4>");
                    path.push_array_index(key.as_str());
                    match item.bind(path, loader) {
                        Ok(_) => {
                            result = result.or(Ok(()));
                        }
                        Err(e) => {
                            if e != NotFound {
                                return Err(e);
                            }
                        }
                    }
                    path.pop_array_index();
                }
            }
            Custom(indices) => {
                for (i, index) in indices.iter().enumerate() {
                    path.push_array_index(index);
                    match self.items[i].bind(path, loader) {
                        Ok(_) => {
                            result = result.or(Ok(()));
                        }
                        Err(e) => {
                            if e != NotFound {
                                return Err(e);
                            }
                        }
                    }
                    path.pop_array_index();
                }
            }
        }

        result
    }
}

pub struct ArrayRefBinder<'a, T> {
    /// The reference key for the array.
    array_ref: &'static str,
    /// Optional prefix to strip from the array index.
    prefix: Option<&'static str>,
    /// The value to bind to the array reference.
    value: &'a mut T,
}

impl<'a, T> ArrayRefBinder<'a, T> {
    pub fn new(array_ref: &'static str, prefix: Option<&'static str>, value: &'a mut T) -> Self {
        Self {
            array_ref,
            prefix,
            value,
        }
    }
}

/// ConfigBinder implementation for binding array references.
/// This binder allows for binding to a specific array reference in the configuration backend,
/// with an optional prefix to strip from the array index.
impl<'a, T, U, B> ConfigBinder<T, U> for ArrayRefBinder<'a, B>
where
    B: ConfigBinder<T, U>,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let index: crate::str_ty!() = loader.load_str_value(path.current_path())?;
        let key = if let Some(prefix) = self.prefix {
            index.strip_prefix(prefix)
        } else {
            Some(index.as_str())
        };

        if let Some(key) = key {
            let mut ref_path = T::new();
            ref_path.push(self.array_ref);
            ref_path.push_array_index(key);
            self.value.bind(&mut ref_path, loader)?;
        }

        Ok(())
    }
}

/// ConfigBinder implementation for boolean values.
impl<T, U> ConfigBinder<T, U> for bool
where
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let key = path.current_path();
        *self = loader.load_bool_value(key)?;

        Ok(())
    }
}

trait Numeric {}
impl Numeric for i32 {}
impl Numeric for u32 {}
impl Numeric for u16 {}
impl Numeric for i16 {}
impl Numeric for i8 {}
impl Numeric for u8 {}

/// ConfigBinder implementation for numeric types that can be converted from i32.
impl<N, T, U> ConfigBinder<T, U> for N
where
    N: Numeric + TryFrom<i32>,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let key = path.current_path();
        let value = loader.load_number_value(key)?;
        *self = value
            .try_into()
            .map_err(|_| crate::error::Error::Overflow)?;

        Ok(())
    }
}

/// ConfigBinder implementation for `Option<V>` where `V` is another type that implements `ConfigBinder`.
/// This allows for optional configuration values that may or may not be present in the configuration backend.
/// If the value is not found, it sets the option to `None`
impl<T, U, V> ConfigBinder<T, U> for Option<V>
where
    T: BindPath,
    U: PropertyLoader,
    V: ConfigBinder<T, U>,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let mut maybe_value: MaybeUninit<V> = MaybeUninit::zeroed();

        // SAFETY: If binding fails we return Err()
        let value = unsafe { maybe_value.assume_init_mut() };
        match value.bind(path, loader) {
            Ok(()) => {
                // SAFETY: We have successfully bound `maybe_value` and it is safe to assume it is initialized now.
                *self = Some(unsafe { maybe_value.assume_init() });
                Ok(())
            }
            Err(crate::error::Error::NotFound) => {
                *self = None;
                Ok(())
            }
            Err(crate::error::Error::Required) => {
                *self = None;
                Ok(())
            }
            Err(e) => {
                // If any other error occurs, we propagate it
                Err(e)
            }
        }
    }
}
