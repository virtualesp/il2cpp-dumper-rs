#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid metadata file: {0}")]
    InvalidMetadata(String),

    #[error("Unsupported metadata version: {0}")]
    UnsupportedVersion(i32),

    #[error("Address 0x{0:x} not in any segment")]
    AddressNotMapped(u64),

    #[error("Invalid binary format: {0}")]
    InvalidFormat(String),

    #[error("Read out of bounds: offset 0x{offset:x}, size {size}")]
    OutOfBounds { offset: u64, size: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
