//! Letting an outside agent use Rune Studio's eyes and hands instead of the built-in model.
//!
//! The lease (who may drive) is always compiled, because the built-in agent and Stop consult it.
//! The server that agents connect to exists only in builds made with `--features agent-bridge`.
//! See docs/AGENT-GUIDE.md for the contract agents are written against.

pub mod identity;
pub mod lease;

/// The file name of one of our programs on this OS (`rune-studio` is `rune-studio.exe` on Windows).
#[cfg(feature = "agent-bridge")]
pub fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

#[cfg(feature = "agent-bridge")]
pub mod agents;
#[cfg(feature = "agent-bridge")]
pub mod cli;
#[cfg(feature = "agent-bridge")]
pub mod mcp;
#[cfg(all(feature = "agent-bridge", unix))]
pub mod path_link;
#[cfg(feature = "agent-bridge")]
pub mod server;
#[cfg(feature = "agent-bridge")]
pub mod skill;
#[cfg(feature = "agent-bridge")]
pub mod token;
