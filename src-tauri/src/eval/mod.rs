//! The evaluation harness: a fixed suite of desktop tasks run through the built-in agent, with the
//! outcome of each decided by looking at the machine, never by what the model says.
//!
//! Compiled only with `--features eval`, and only active when `RUNE_EVAL_SUITE` and
//! `RUNE_EVAL_OUT` are both set. A normal build has no way to start a task from a file.

pub mod check;
pub mod spec;

#[cfg(test)]
mod tests;
