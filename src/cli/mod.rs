//! Command-line interface layer.
//!
//! This module handles argument parsing and command dispatch using clap.

mod args;
mod handlers;

pub use args::{
    Cli, Commands, DiagramArg, FormatArg, GenerateArgs, StatsArgs, ThemeArg, ValidateArgs, WikiArgs,
};
pub use handlers::run;
