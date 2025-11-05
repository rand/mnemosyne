//! CLI command handlers
//!
//! This module contains all the command handlers for the mnemosyne CLI.
//! Each subcommand is implemented in its own module for better organization.

pub mod helpers;
pub mod serve;
pub mod api_server;
pub mod init;
pub mod export;
pub mod status;
pub mod edit;
pub mod tui;
pub mod config;
pub mod secrets;
pub mod orchestrate;
pub mod remember;
pub mod recall;
pub mod embed;
pub mod models;
pub mod evolve;
pub mod artifact;
pub mod doctor;
