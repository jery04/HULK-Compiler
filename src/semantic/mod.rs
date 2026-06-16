//! Semantic analysis for the HULK compiler.
//!
//! This module performs type checking and semantic validation of the AST.
//! It ensures that:
//! - All identifiers are defined in the current scope.
//! - Type annotations are consistent with inferred types.
//! - Function/method calls have correct arity and argument types.
//! - Inheritance and protocol implementation are valid.
//!
//! The main entry point is [`check_program`], which returns a list of semantic errors.

pub mod checker;
pub mod context;

#[cfg(test)]
mod tests;

pub use checker::check_program;