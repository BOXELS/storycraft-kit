//! STORYCRAFT core: everything `sc` and `storycraft-mcp` share.
//!
//! The Craft apps are driven only through their own command-line tools (`<app>-cli`); nothing here
//! links app crates.

pub mod apps;
pub mod bug;
pub mod clients;
pub mod doctor;
pub mod engine;
pub mod error;
pub mod flags;
pub mod pack;
pub mod paths;
pub mod recipe;
pub mod verify;

pub use error::{Error, Result};
