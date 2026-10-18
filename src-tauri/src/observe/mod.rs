//! Seeing the screen: capture, downscale and encode.
//!
//! Until the agent loop lands (M2) the only callers are the debug commands, which exist only in
//! debug builds, so a release build sees this module as unused.
#![cfg_attr(not(debug_assertions), allow(dead_code))]

pub mod screenshot;
