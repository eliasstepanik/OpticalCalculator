//! `optical-calculator` — CLI entry point.
//!
//! Arg parsing, config loading, dispatch, and exit codes land in Chunk 17.

mod common;
mod config;
mod designs;
#[cfg(test)]
mod examples_validate;
mod error;
mod glass;
mod output;
mod refractor_chromatic;
mod refractor_chromatic_solve;
mod render;
mod request;

fn main() {}
