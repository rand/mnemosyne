//! RPC server module (gRPC with Tonic)
//!
//! This module is only compiled when the `rpc` feature is enabled.
//! It provides a gRPC server for remote access to mnemosyne's memory system.

#![cfg(feature = "rpc")]

pub mod generated;
pub mod server;
pub mod services;
pub mod errors;

pub use server::RpcServer;
