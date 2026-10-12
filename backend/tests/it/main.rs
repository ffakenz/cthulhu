//! Integration tests: one crate, with modules that mirror `src`, and `cli`, which drives the
//! binary.

mod support;

mod cli;
mod core;
mod csv_io;
mod domain;
mod memory;
mod run;
