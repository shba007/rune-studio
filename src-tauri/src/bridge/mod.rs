//! Letting an outside agent use Rune Studio's eyes and hands instead of the built-in model.
//!
//! The lease (who may drive) is always compiled, because the built-in agent and Stop consult it.
//! The server that agents connect to exists only in builds made with `--features agent-bridge`.
//! See docs/AGENT_GUIDE.md for the contract agents are written against.

pub mod lease;

#[cfg(feature = "agent-bridge")]
pub mod cli;
#[cfg(feature = "agent-bridge")]
pub mod mcp;
#[cfg(feature = "agent-bridge")]
pub mod server;
#[cfg(feature = "agent-bridge")]
pub mod skill;
#[cfg(feature = "agent-bridge")]
pub mod token;
