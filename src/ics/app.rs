//! Main ICS application
//!
//! Standalone ICS application that can be run with `mnemosyne --ics`

use super::{IcsConfig, editor::IcsEditor};
use anyhow::Result;

/// Main ICS application
pub struct IcsApp {
    /// Configuration
    config: IcsConfig,
    /// Editor instance
    editor: IcsEditor,
}

impl IcsApp {
    /// Create new ICS application
    pub fn new(config: IcsConfig) -> Self {
        Self {
            config,
            editor: IcsEditor::new(),
        }
    }

    /// Run the ICS application
    pub async fn run(&mut self) -> Result<()> {
        // TODO: Implement terminal loop
        // - Initialize terminal with crossterm
        // - Set up event loop
        // - Render UI with ratatui
        // - Handle input events
        Ok(())
    }
}
