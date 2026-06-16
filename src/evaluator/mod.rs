//! Evaluator module for HULK arithmetic expressions.
//!
//! This module provides a simple interpreter for arithmetic expressions,
//! supporting basic operations (`+`, `-`, `*`, `/`, `%`, `^`), unary negation,
//! function calls (`sin`, `cos`, `sqrt`, `exp`, `log`), and constants (`PI`).
//! The evaluator parses a source string and returns an `Option<i64>`.
//!
//! The main entry point is [`evaluate`].


pub mod evaluator;
#[cfg(test)]
mod tests;
