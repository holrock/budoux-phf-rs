//! A Rust implementation of [BudouX](https://github.com/google/budoux), a
//! machine learning based line break organizer.
//!
//! The language models are embedded at compile time as perfect hash maps, so
//! there is no dictionary to load at runtime.
//!
//! Start from [`Parser`]: build one with a bundled model (for example
//! [`Parser::japanese_parser`]) or with your own via [`Parser::new`], then call
//! [`Parser::parse`] to get the chunks as a `Vec` (requires the `alloc` or
//! `std` feature), or [`Parser::parse_with`] to receive them through a callback
//! without allocating.
#![no_std]
#![forbid(unsafe_code)]
#[cfg(feature = "alloc")]
extern crate alloc;

mod model;
pub use model::{Model, ScoreMap};
#[cfg(feature = "ja")]
mod model_ja;
#[cfg(feature = "ja_knbc")]
mod model_ja_knbc;
#[cfg(feature = "th")]
mod model_th;
#[cfg(feature = "zh_hans")]
mod model_zh_hans;
#[cfg(feature = "zh_hant")]
mod model_zh_hant;

mod parser;
pub use parser::Parser;
