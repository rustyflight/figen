use crate::error::Error::NotFound;
use crate::loader::PropertyLoader;
use crate::BindPath;

pub const MAX_ARRAY_SIZE: usize = 1024;
const MAX_ARRAY_KEY_SIZE_STR: usize = 4;

pub trait ConfigBinder<T, U>
where
    Self: Sized,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()>;
}

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
    OneIndexed,
    Custom(&'a [&'a str]),
}


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
        assert!(self.items.len() <= MAX_ARRAY_SIZE, "Array size exceeds maximum allowed size of {}", MAX_ARRAY_SIZE);

        let mut result = Err(NotFound);
        match self.mode {
            ZeroIndexed => {
                for (i, item) in self.items.iter_mut().enumerate() {
                    let key: heapless::String<{ MAX_ARRAY_KEY_SIZE_STR }> = heapless::String::try_from(i as u32).expect("Index too large for heapless::String<4>");
                    path.push(key.as_str());
                    match item.bind(path, loader) {
                        Ok(_) => { result = result.or(Ok(())); }
                        Err(e) => {
                            if e != NotFound {
                                return Err(e);
                            }
                        }
                    }

                    path.pop();
                }
            }
            OneIndexed => {
                for (i, item) in self.items.iter_mut().enumerate() {
                    let key: heapless::String<{ MAX_ARRAY_KEY_SIZE_STR }> = heapless::String::try_from((i + 1) as u32).expect("Index too large for heapless::String<4>");
                    path.push(key.as_str());
                    match item.bind(path, loader) {
                        Ok(_) => { result = result.or(Ok(())); }
                        Err(e) => {
                            if e != NotFound {
                                return Err(e);
                            }
                        }
                    }
                    path.pop();
                }
            }
            Custom(indices) => {
                for (i, index) in indices.iter().enumerate() {
                    path.push(index);
                    match self.items[i].bind(path, loader) {
                        Ok(_) => { result = result.or(Ok(())); }
                        Err(e) => {
                            if e != NotFound {
                                return Err(e);
                            }
                        }
                    }
                    path.pop();
                }
            }
        }

        result
    }
}

pub struct ArrayRefBinder<'a, T> {
    array_ref: &'static str,
    prefix: Option<&'static str>,
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

impl<'a, T, U, B> ConfigBinder<T, U> for ArrayRefBinder<'a, B>
where
    B: ConfigBinder<T, U>,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        #[cfg(feature = "std")]
        let index = loader.load_str_value(path.current_path())?;
        #[cfg(not(feature = "std"))]
        let index = loader.load_str_value::<11>(path.current_path())?; // TODO: Hardcoded 11 char long string as key! Not ideal
        let key = if let Some(prefix) = self.prefix {
            index.strip_prefix(prefix)
        } else {
            Some(index.as_str())
        };

        if let Some(key) = key {
            let mut ref_path = T::new();
            ref_path.push(self.array_ref);
            ref_path.push(key);

            self.value.bind(&mut ref_path, loader)?;
        }


        Ok(())
    }
}

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

impl<T, U> ConfigBinder<T, U> for f32
where
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let key = path.current_path();
        let num = loader.load_number_value(key)?;
        *self = num as f32;

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


impl<N, T, U> ConfigBinder<T, U> for N
where
    N: Numeric + TryFrom<i32>,
    T: BindPath,
    U: PropertyLoader,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let key = path.current_path();
        let value = loader.load_number_value(key)?;
        *self = value.try_into().map_err(|_| crate::error::Error::Overflow)?;

        Ok(())
    }
}

impl<T, U, V> ConfigBinder<T, U> for Option<V>
where
    T: BindPath,
    U: PropertyLoader,
    V: ConfigBinder<T, U> + Default,
{
    fn bind(&mut self, path: &mut T, loader: &U) -> crate::error::Result<()> {
        let mut value = V::default();

        match value.bind(path, loader) {
            Ok(()) => {
                *self = Some(value);
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