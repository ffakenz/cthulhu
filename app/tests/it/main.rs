//! Integration tests: one crate, with `run`, which holds the engine to equations between its runs
//! through the CSV source; `domain`, which holds the laws of money; and `cli`, which drives the
//! binary.

mod support;

mod cli;
mod domain;
mod run;
