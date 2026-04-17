//! # poker
//!
//! `poker` is a terminal-based poker game written in Rust.
//!
//! The project supports player account creation, PIN-based login with hidden
//! input, persistent balances, and basic player statistics stored in SQLite.
//! Hands are evaluated using standard 5-card poker rules.
//!
//! ## Modules
//!
//! - [`cards`] – Cards, ranks, and suits
//! - [`ranking`] – Hand evaluation and comparison
//! - [`player`] – Player data model
//! - [`db`] – Database + authentication
//! - [`game`] – Game loop and round flow
//! - [`console_ui`] – Terminal input/output helpers
//!
//! `main.rs` is the binary entry point and drives the UI flow.

pub mod db;
pub mod game;
pub mod player;
