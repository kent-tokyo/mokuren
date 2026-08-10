//! The v0.1 vertical slice (AGENTS.md section 1): harmonize a melody,
//! explain the result, and ask why an alternative wasn't chosen.
//!
//! Run with: cargo run --example basic

use mokuren::prelude::*;

fn main() -> Result<()> {
    let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4")?;

    let result = Composer::new()
        .key(Key::C_MAJOR)
        .style(Style::CommonPractice)
        .voices(Voices::SATB)
        .search(BeamSearch::new().width(32))
        .harmonize(melody)?;

    println!("{}", result.explain());

    // Position 2 has a previous chord to reason against, so its `why()`
    // carries real voice-leading/harmonic-function/cadence reasons —
    // unlike position 0, which has nothing yet to compare against.
    println!("\n{}", result.why(Position::new(2))?);

    // iii (E-G-B) also contains this position's soprano note (G4), so it
    // was a real evaluated alternative to V.
    println!("\n{}", result.why_not(Position::new(2), RomanNumeral::III)?);

    println!("\n{}", result.diagnostics());

    Ok(())
}
