//! Lexer module for the HULK compiler.
//!
//! This module provides the token definitions and a streaming lexer
//! that produces tokens with source spans and collects lexical errors.
//! The public interface is [`Token`], [`TokenStream`], [`Pos`], [`Span`], and [`LexError`].


pub mod lexer;

#[cfg(test)]
#[path = "test.rs"]
mod test;
