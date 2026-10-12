//! Integration tests: one crate, with modules that mirror the modules of `engine`, `store` and
//! `backend`, and `cli`, which drives the binary.

mod support;

mod cli;
mod core;
mod csv_io;
mod domain;
mod memory;
mod run;
