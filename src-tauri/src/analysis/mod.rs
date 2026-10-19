//! Visual analysis: the user presses a button in the pill, Rune looks at the current screen, asks
//! the omni model about it and shows the answer in its own window as numbered regions on the
//! screenshot, each with an evidence line. It is separate from the agent loop and never acts on the
//! desktop. See `docs/ARCHITECTURE.md`, "Visual analysis".

pub mod report;

pub use report::{Analysis, Finding, Kind, Status};
