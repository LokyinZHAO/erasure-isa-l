mod bind;
pub mod erasure;
pub mod galois;

pub use bind::ec;
pub use bind::gf;

/// The `Error` enum defines the possible errors that this crate can occur.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// TooManyErasure: The number of erasures is larger than the maximum allowed,
    /// and the lost data cannot be recovered.
    #[error("Too Many Erased Blocks: {0} erased, up to {1} allowed")]
    TooManyErasures(usize, usize),
    /// InvalidArguments: The the input is invalid.
    #[error("Invalid Arguments: {0}")]
    InvalidArguments(String),
    /// InternalError: An internal error caused by libisa-l.
    #[error("Internal Error: {0}")]
    InternalError(String),
    /// Other: Other errors that are not covered by the above.
    #[error("Error: {0}")]
    Other(String),
}

#[allow(dead_code)]
impl Error {
    fn too_many_erasures(erasures: usize, max_erasures: usize) -> Self {
        Self::TooManyErasures(erasures, max_erasures)
    }

    fn invalid_arguments(msg: impl Into<String>) -> Self {
        Self::InvalidArguments(msg.into())
    }

    fn internal_error(msg: impl Into<String>) -> Self {
        Self::InternalError(msg.into())
    }

    fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
