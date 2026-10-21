pub mod identity;
pub mod lease;

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
