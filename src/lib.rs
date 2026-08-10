//! mokuren — a fast, explainable symbolic composition engine for
//! exploring music-theoretic decisions.
//!
//! v0.1's vertical slice: given a soprano melody, search Common Practice
//! SATB harmonizations and explain *why* each chord was chosen and *why
//! not* the alternatives. See `PLAN.md` for scope and phasing.
//!
//! ```
//! use mokuren::prelude::*;
//!
//! let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4")?;
//!
//! let result = Composer::new()
//!     .key(Key::C_MAJOR)
//!     .style(Style::CommonPractice)
//!     .voices(Voices::SATB)
//!     .search(BeamSearch::new().width(8))
//!     .harmonize(melody)?;
//!
//! // Every position resolved to a chord that passed every hard rule.
//! assert!(result.decisions.iter().all(|d| d.selected_candidate().is_valid()));
//! # Ok::<(), MokurenError>(())
//! ```

pub mod chord;
pub mod compose;
pub mod diagnostics;
pub mod error;
pub mod explain;
pub mod generate;
pub mod key;
pub mod melody;
pub mod pitch;
pub mod rules;
pub mod score;
pub mod search;
pub mod voice;

pub use error::MokurenError;

/// The common entry points: `use mokuren::prelude::*;` and go straight to
/// `Composer::new()...harmonize(melody)?`.
pub mod prelude {
    pub use crate::chord::{ChordInversion, ChordQuality, HarmonicFunction, RomanNumeral};
    pub use crate::compose::{Composer, Voices};
    pub use crate::error::{MokurenError, Result};
    pub use crate::explain::{Decision, HarmonizationResult};
    pub use crate::key::Key;
    pub use crate::melody::{Melody, Position};
    pub use crate::rules::Style;
    pub use crate::search::BeamSearch;
}
