//! CLI command handlers
//!
//! This module contains all the command handlers for the mnemosyne CLI.
//! Each subcommand is implemented in its own module for better organization.

pub mod api_server;
pub mod artifact;
pub mod config;
pub mod doctor;
pub mod edit;
pub mod embed;
pub mod event_bridge;
pub mod event_helpers;
pub mod evolve;
pub mod export;
pub mod graph;
pub mod helpers;
pub mod init;
pub mod interactive;
pub mod internal;
pub mod models;
pub mod orchestrate;
pub mod peer;
pub mod recall;
pub mod remember;
pub mod secrets;
pub mod serve;
pub mod status;
pub mod tui;
pub mod update;
