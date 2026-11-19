//! Offline WAV equalizer for the factory Creative G6 EQ preset curves.
//!
//! Scope is deliberately narrow: 10-band RBJ peaking biquads plus a pre-amp
//! scalar. No SBX / Crystalizer / Bass / Dialog+ / Smart Volume / Scout and no
//! reverb is implemented, and nothing here claims to reproduce the device DSP.

pub mod biquad;
pub mod cli;
pub mod engine;
pub mod error;
pub mod preset;
pub mod wav;

pub use error::AppError;
