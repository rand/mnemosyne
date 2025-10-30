//! Terminal setup and management

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

/// Terminal configuration
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Enable mouse support
    pub mouse_enabled: bool,

    /// Use alternate screen
    pub alternate_screen: bool,

    /// Enable raw mode
    pub raw_mode: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            mouse_enabled: true,
            alternate_screen: true,
            raw_mode: true,
        }
    }
}

/// Terminal manager wrapping ratatui Terminal
pub struct TerminalManager {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    config: TerminalConfig,
}

impl TerminalManager {
    /// Initialize terminal with configuration
    pub fn new(config: TerminalConfig) -> Result<Self> {
        // Enable raw mode
        if config.raw_mode {
            enable_raw_mode()?;
        }

        let mut stdout = io::stdout();

        // Enter alternate screen
        if config.alternate_screen {
            execute!(stdout, EnterAlternateScreen)?;
        }

        // Enable mouse
        if config.mouse_enabled {
            execute!(stdout, EnableMouseCapture)?;
        }

        // Create terminal
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal, config })
    }

    /// Get mutable reference to terminal
    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Get terminal size
    pub fn size(&self) -> Result<(u16, u16)> {
        let rect = self.terminal.size()?;
        Ok((rect.width, rect.height))
    }

    /// Clear terminal
    pub fn clear(&mut self) -> Result<()> {
        self.terminal.clear()?;
        Ok(())
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        // Restore terminal state
        if self.config.raw_mode {
            let _ = disable_raw_mode();
        }

        if self.config.mouse_enabled {
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }

        if self.config.alternate_screen {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        }
    }
}
