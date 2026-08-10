use std::fmt;

/// Errors returned at mokuren's public API boundaries.
///
/// Internal computation never panics; parsing and lookups that can fail
/// on bad input return this instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MokurenError {
    /// A melody, pitch, or other text input could not be parsed.
    Parse(String),
    /// A `Position` referenced a step that doesn't exist in the result.
    UnknownPosition(usize),
    /// `why_not` was asked about a candidate that was never evaluated
    /// at the given position.
    UnknownAlternative(String),
    /// The search produced no valid harmonization at all.
    NoValidHarmonization,
}

impl fmt::Display for MokurenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MokurenError::Parse(msg) => write!(f, "parse error: {msg}"),
            MokurenError::UnknownPosition(pos) => write!(f, "unknown position: {pos}"),
            MokurenError::UnknownAlternative(msg) => write!(f, "unknown alternative: {msg}"),
            MokurenError::NoValidHarmonization => write!(f, "no valid harmonization found"),
        }
    }
}

impl std::error::Error for MokurenError {}

pub type Result<T> = std::result::Result<T, MokurenError>;
