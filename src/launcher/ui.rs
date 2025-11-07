//! Launch UX - Clean, Playful Startup Experience
//!
//! This module provides refined user-facing output during Mnemosyne launch,
//! replacing verbose INFO logs with a clean, informative, and playful experience.

use crate::icons;
use rand::seq::SliceRandom;
use std::io::{self, Write};

/// ASCII art banner for Mnemosyne
const BANNER: &str = r#"
███╗   ███╗███╗   ██╗███████╗███╗   ███╗ ██████╗ ███████╗██╗   ██╗███╗   ██╗███████╗
████╗ ████║████╗  ██║██╔════╝████╗ ████║██╔═══██╗██╔════╝╚██╗ ██╔╝████╗  ██║██╔════╝
██╔████╔██║██╔██╗ ██║█████╗  ██╔████╔██║██║   ██║███████╗ ╚████╔╝ ██╔██╗ ██║█████╗
██║╚██╔╝██║██║╚██╗██║██╔══╝  ██║╚██╔╝██║██║   ██║╚════██║  ╚██╔╝  ██║╚██╗██║██╔══╝
██║ ╚═╝ ██║██║ ╚████║███████╗██║ ╚═╝ ██║╚██████╔╝███████║   ██║   ██║ ╚████║███████╗
╚═╝     ╚═╝╚═╝  ╚═══╝╚══════╝╚═╝     ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═╝  ╚═══╝╚══════╝
"#;

/// Tagline displayed below banner
const TAGLINE: &str = "Intelligent Agentic Memory and Orchestration";

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
    "Initializing agent substrate",
    "Harmonizing vector embeddings",
    "Bootstrapping semantic networks",
    "Activating memory traces",
    "Synchronizing thought streams",
    "Priming knowledge graphs",
    "Energizing cognitive pathways",
    "Indexing conceptual spaces",
    "Weaving context threads",
    "Awakening neural ensembles",
    "Crystallizing insights",
    "Tuning attention mechanisms",
];

/// Fun glyphs for transition animation (Nerd Font icons)
const TRANSITION_GLYPHS: &[&str] = &[
    "\u{f0eb}", // fa-lightbulb
    "\u{f135}", // fa-rocket
    "\u{f0e7}", // fa-bolt (lightning)
    "\u{f005}", // fa-star
    "\u{f021}", // fa-refresh (sync)
    "\u{f013}", // fa-gear
    "\u{f5dc}", // md-brain
    "\u{f0c1}", // fa-link
];

/// Particle explosion animation frames (displayed above Claude Code character)
/// Designed to emanate upward from the Space Invader's head in a fountain/cone shape
/// Positioned at bottom-left to align with Space Invader icon
const EXPLOSION_FRAME_1: &str = r#"
      ∗
    ·   ·
      ✦
"#;

const EXPLOSION_FRAME_2: &str = r#"
 ⋆         ✧
  ·     ∗   ˙     ·
    ∘     ✦     ∘
      •  ○  •
          ∗
"#;

const EXPLOSION_FRAME_3: &str = r#"
✧                       ⋆
 ·       ∗           ˙       ·
   ∘           ⋅       ∗           ∘
 ·     •       ✦       •     ·
      ∗     ○   ○   ○     ∗
        ·   •   ∗   •   ·
              ∗ ✦ ∗
                ∗
"#;

/// ANSI color codes for banner gradient
mod colors {
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BLUE: &str = "\x1b[34m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const RESET: &str = "\x1b[0m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const YELLOW: &str = "\x1b[33m";
}

/// Launch progress tracker
pub struct LaunchProgress;

impl LaunchProgress {
    /// Create a new launch progress tracker
    pub fn new() -> Self {
        Self
    }

    /// Display the full banner with gradient colors
    pub fn show_banner(&self) {
        let lines: Vec<&str> = BANNER.trim().lines().collect();
        let gradient_colors = [
            colors::BRIGHT_BLUE,
            colors::BLUE,
            colors::CYAN,
            colors::BRIGHT_CYAN,
            colors::BRIGHT_MAGENTA,
            colors::MAGENTA,
        ];

        println!(); // Top spacing
        for (i, line) in lines.iter().enumerate() {
            let color = gradient_colors[i % gradient_colors.len()];
            println!("{}{}{}", color, line, colors::RESET);
        }

        // Centered tagline in italic yellow
        println!(
            "{}{}{}{}",
            colors::ITALIC,
            colors::YELLOW,
            center_text(TAGLINE, 80),
            colors::RESET
        );
        println!(); // Bottom spacing
    }

    /// Display the main launch header (legacy - now shows banner)
    pub fn show_header(&self, version: &str) {
        self.show_banner();
        println!("   v{}\n", version);
    }

    /// Show a playful loading message (now with random selection)
    pub fn show_loading_message(&self) {
        let mut rng = rand::thread_rng();
        let message = LOADING_MESSAGES.choose(&mut rng).unwrap_or(&"Initializing");
        print!("   {}  {}...", icons::system::gear(), message);
        io::stdout().flush().ok();
    }

    /// Cycle through multiple random loading messages
    pub fn cycle_loading_messages(&self, count: usize) {
        let mut rng = rand::thread_rng();
        let mut messages = LOADING_MESSAGES.to_vec();
        messages.shuffle(&mut rng);

        for message in messages.iter().take(count) {
            print!(
                "\r   {}  {}...                    ",
                icons::system::gear(),
                message
            );
            io::stdout().flush().ok();
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        print!("\r"); // Clear the line
        io::stdout().flush().ok();
    }

    /// Display 3 lines of loading messages, each randomizing independently before settling
    pub fn show_multiline_loading(&self) {
        let mut rng = rand::thread_rng();

        // Prepare 3 separate message pools
        let mut messages1 = LOADING_MESSAGES.to_vec();
        let mut messages2 = LOADING_MESSAGES.to_vec();
        let mut messages3 = LOADING_MESSAGES.to_vec();
        messages1.shuffle(&mut rng);
        messages2.shuffle(&mut rng);
        messages3.shuffle(&mut rng);

        // Pick final messages (different for each line)
        let final1 = messages1[0];
        let final2 = messages2[0];
        let final3 = messages3[0];

        // Number of randomization cycles per line (~250ms total at 50ms/cycle)
        let cycles_per_line = 5;

        // Animate all 3 lines simultaneously
        for cycle in 0..cycles_per_line {
            // Move cursor up to overwrite previous lines (except first cycle)
            if cycle > 0 {
                print!("\x1b[3A"); // ANSI escape: move cursor up 3 lines
            }

            let msg1 = if cycle < cycles_per_line - 1 {
                messages1[cycle % messages1.len()]
            } else {
                final1
            };
            let msg2 = if cycle < cycles_per_line - 1 {
                messages2[cycle % messages2.len()]
            } else {
                final2
            };
            let msg3 = if cycle < cycles_per_line - 1 {
                messages3[cycle % messages3.len()]
            } else {
                final3
            };

            // Print lines with padding to clear previous content
            println!(
                "\r   {}  {}...{}",
                icons::system::gear(),
                msg1,
                " ".repeat(50)
            );
            println!(
                "\r   {}  {}...{}",
                icons::system::gear(),
                msg2,
                " ".repeat(50)
            );
            println!(
                "\r   {}  {}...{}",
                icons::system::gear(),
                msg3,
                " ".repeat(50)
            );

            io::stdout().flush().ok();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// Show animated glyph transition between mnemosyne and Claude Code UI
    pub fn show_transition(&self) {
        println!(); // Blank line after loading messages
        println!(); // Second blank line

        let gradient_colors = [
            colors::BRIGHT_BLUE,
            colors::CYAN,
            colors::BRIGHT_MAGENTA,
            colors::MAGENTA,
            colors::YELLOW,
        ];

        // Animate glyphs cycling through colors
        for cycle in 0..8 {
            print!("\r   ");

            for i in 0..6 {
                let glyph_idx = (cycle + i) % TRANSITION_GLYPHS.len();
                let color = gradient_colors[i % gradient_colors.len()];
                print!("{}{} ", color, TRANSITION_GLYPHS[glyph_idx]);
            }

            print!("{}", colors::RESET);
            io::stdout().flush().ok();
            std::thread::sleep(std::time::Duration::from_millis(60));
        }

        // Clear the line and add spacing
        println!("\r{}", " ".repeat(80));
        println!(); // Blank space before Claude Code UI
        println!(); // Extra blank space
    }

    /// Show colorful particle explosion animation (20% chance on startup)
    /// Positioned at bottom-left to align with Space Invader icon in Claude Code UI
    pub fn show_explosion_animation(&self) {
        let gradient_colors = [
            colors::BRIGHT_BLUE,
            colors::BLUE,
            colors::CYAN,
            colors::BRIGHT_CYAN,
            colors::BRIGHT_MAGENTA,
            colors::MAGENTA,
        ];

        let frames = [EXPLOSION_FRAME_1, EXPLOSION_FRAME_2, EXPLOSION_FRAME_3];

        // Get terminal dimensions for positioning
        let (row, col) = if let Ok((_, height)) = crossterm::terminal::size() {
            // Position at bottom-left: 12 rows from bottom, 8 columns from left
            // This aligns with where Claude Code renders the Space Invader
            let target_row = height.saturating_sub(12);
            (target_row, 8u16)
        } else {
            // Fallback: position near bottom if terminal size unavailable
            (20u16, 8u16)
        };

        // Display each frame at the fixed bottom-left position
        for frame in &frames {
            let lines: Vec<&str> = frame.trim().lines().collect();

            // Position cursor at bottom-left before rendering each frame
            print!("\x1b[{};{}H", row, col);

            for (i, line) in lines.iter().enumerate() {
                let color = gradient_colors[i % gradient_colors.len()];
                // Use absolute positioning for each line to avoid cursor drift
                print!("\x1b[{};{}H{}{}{}", row + i as u16, col, color, line, colors::RESET);
            }

            io::stdout().flush().ok();
            std::thread::sleep(std::time::Duration::from_millis(100));

            // For subsequent frames, reposition cursor to same starting point
            // (overwrite previous frame in place)
        }

        // Move cursor below explosion for clean transition
        print!("\x1b[{};1H", row + 10);
        println!();
    }

    /// Clear the current line and show completion
    pub fn show_step_complete(&self, step_name: &str) {
        print!("\r   {} {}\n", icons::status::success(), step_name);
        io::stdout().flush().ok();
    }

    /// Show configuration details with agent names
    pub fn show_config(&self, db_path: &str, agent_names: &[&str]) {
        println!("   Database: {}", db_path);
        println!(
            "   Agents: {}: {}\n",
            agent_names.len(),
            agent_names.join(", ")
        );
    }

    /// Show a brief status message
    pub fn show_status(&self, message: &str) {
        println!("   {}", message);
    }

    /// Show completion banner
    pub fn show_completion(&self) {
        println!("\n   {} Ready\n", icons::status::ready());
    }

    /// Show error with context
    pub fn show_error(&self, error: &str) {
        eprintln!("\n   {} Error: {}\n", icons::status::error(), error);
    }
}

impl Default for LaunchProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to center text in a given width
fn center_text(text: &str, width: usize) -> String {
    let text_len = text.len();
    if text_len >= width {
        return text.to_string();
    }
    let padding = (width - text_len) / 2;
    format!("{}{}", " ".repeat(padding), text)
}

/// Quick helper to show a simple launch header with config
pub fn show_launch_header(version: &str, db_path: &str, agent_names: &[&str]) {
    let progress = LaunchProgress::new();
    progress.show_header(version);
    progress.show_config(db_path, agent_names);
}

/// Quick helper to show a loading step
pub fn show_loading_step(step_name: &str) {
    let progress = LaunchProgress::new();
    progress.show_loading_message();
    // Give it a moment to display
    std::thread::sleep(std::time::Duration::from_millis(50));
    progress.show_step_complete(step_name);
}

/// Check for updates and show notification if available
/// This is non-blocking and will timeout after 3 seconds
pub async fn check_and_show_updates() {
    use crate::version_check::VersionChecker;

    // Create version checker
    let checker = match VersionChecker::new() {
        Ok(c) => c,
        Err(_) => return, // Silently fail if we can't create checker
    };

    // Check all tools with 3-second timeout
    let check_future = checker.check_all_tools();
    let results = match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        check_future
    ).await {
        Ok(Ok(results)) => results,
        _ => return, // Silently fail on timeout or error
    };

    // Filter to only tools with updates available
    let updates: Vec<_> = results.iter()
        .filter(|info| info.update_available)
        .collect();

    if !updates.is_empty() {
        println!();
        println!("   {} Updates available:", icons::status::info());
        for info in &updates {
            if let (Some(installed), Some(latest)) = (&info.installed, &info.latest) {
                println!(
                    "   {} {}: {} → {}",
                    icons::status::warning(),
                    info.tool.display_name(),
                    installed,
                    latest
                );
            }
        }
        println!("   Run 'mnemosyne update' to update all tools");
        println!();
    }

    // Check for missing tools
    let missing: Vec<_> = results.iter()
        .filter(|info| !info.is_installed)
        .collect();

    if !missing.is_empty() {
        println!("   {} Optional tools not installed:", icons::status::info());
        for info in &missing {
            println!("   {} {}", icons::status::warning(), info.tool.display_name());
        }
        println!("   Run 'mnemosyne update --install' for installation instructions");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_messages_random() {
        let progress = LaunchProgress::new();
        // Just verify we can call show_loading_message multiple times without panic
        for _ in 0..10 {
            progress.show_loading_message();
        }
    }

    #[test]
    fn test_cycle_loading_messages() {
        let progress = LaunchProgress::new();
        // Verify cycling through messages doesn't panic
        progress.cycle_loading_messages(3);
    }

    #[test]
    fn test_launch_progress_creation() {
        let _progress = LaunchProgress::new();
        // Just verify we can create an instance
    }

    #[test]
    fn test_center_text() {
        assert_eq!(center_text("test", 10), "   test");
        assert_eq!(center_text("a", 5), "  a");
        assert_eq!(center_text("toolong", 5), "toolong");
    }

    #[test]
    fn test_banner_display() {
        let progress = LaunchProgress::new();
        // Just verify banner display doesn't panic
        progress.show_banner();
    }
}
