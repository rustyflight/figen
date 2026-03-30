pub type Result<T> = core::result::Result<T, Error>;

/// Error type for configuration operations.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    /// The requested entry is required but not found.
    Required,
    /// The requested entry was not found.
    NotFound,
    /// The entry was found but could not be loaded due to overflow.
    /// For heapless::String this means the entry is too large to fit in the string.
    /// For numbers this means the number is too large to fit in the target type.
    Overflow,
    /// The entry was found but could not be parsed to the desired type.
    ParseError,
}
