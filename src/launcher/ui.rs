//! Launch UX - Clean, Playful Startup Experience
//!
//! This module provides refined user-facing output during Mnemosyne launch,
//! replacing verbose INFO logs with a clean, informative, and playful experience.

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Playful loading messages shown during initialization
const LOADING_MESSAGES: &[&str] = &[
    "Reticulating splines",
    "Wrangling squirrels",
    "Traversing latent space",
    "Pondering the ineffable",
    "Calibrating flux capacitors",
    "Consulting the oracle",
    "Warming up neurons",
    "Aligning chakras",
    "Defragmenting memories",
    "Untangling quantum states",
];

/// Launch progress tracker
pub struct LaunchProgress {
    message_index: Arc<AtomicUsize>,
}

impl LaunchProgress {
    /// Create a new launch progress tracker
    pub fn new() -> Self {
        Self {
            message_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Display the main launch header
    pub fn show_header(&self, version: &str) {
        println!("\n🧠 Launching Mnemosyne Agents :: orchestrating Claude Code");
        println!("   v{}\n", version);
    }

    /// Show a playful loading message
    pub fn show_loading_message(&self) {
        let idx = self.message_index.fetch_add(1, Ordering::Relaxed);
        let message = LOADING_MESSAGES[idx % LOADING_MESSAGES.len()];
        print!("   ⚙  {}...", message);
        io::stdout().flush().ok();
    }

    /// Clear the current line and show completion
    pub fn show_step_complete(&self, step_name: &str) {
        print!("\r   ✓ {}\n", step_name);
        io::stdout().flush().ok();
    }

    /// Show configuration details
    pub fn show_config(&self, db_path: &str, agent_count: usize) {
        println!("   Database: {}", db_path);
        println!("   Agents: {}\n", agent_count);
    }

    /// Show a brief status message
    pub fn show_status(&self, message: &str) {
        println!("   {}", message);
    }

    /// Show completion banner
    pub fn show_completion(&self) {
        println!("\n   ✨ Ready\n");
    }

    /// Show error with context
    pub fn show_error(&self, error: &str) {
        eprintln!("\n   ✗ Error: {}\n", error);
    }
}

impl Default for LaunchProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick helper to show a simple launch header with config
pub fn show_launch_header(version: &str, db_path: &str, agent_count: usize) {
    let progress = LaunchProgress::new();
    progress.show_header(version);
    progress.show_config(db_path, agent_count);
}

/// Quick helper to show a loading step
pub fn show_loading_step(step_name: &str) {
    let progress = LaunchProgress::new();
    progress.show_loading_message();
    // Give it a moment to display
    std::thread::sleep(std::time::Duration::from_millis(50));
    progress.show_step_complete(step_name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_messages_rotate() {
        let progress = LaunchProgress::new();

        // Show that messages rotate
        for i in 0..LOADING_MESSAGES.len() * 2 {
            let idx = i % LOADING_MESSAGES.len();
            assert_eq!(
                progress.message_index.load(Ordering::Relaxed) % LOADING_MESSAGES.len(),
                idx
            );
            progress.show_loading_message();
        }
    }

    #[test]
    fn test_launch_progress_creation() {
        let progress = LaunchProgress::new();
        assert_eq!(progress.message_index.load(Ordering::Relaxed), 0);
    }
}
