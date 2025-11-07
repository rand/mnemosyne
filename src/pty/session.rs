//! PTY session management
#![allow(dead_code)]

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

/// PTY configuration
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// Terminal columns
    pub cols: u16,
    /// Terminal rows
    pub rows: u16,
    /// Shell command to run
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            command: "claude".to_string(),
            args: vec![],
        }
    }
}

/// PTY output chunk
#[derive(Debug, Clone)]
pub struct PtyOutput {
    /// Raw output bytes
    pub data: Vec<u8>,
    /// Whether this is stderr
    pub is_stderr: bool,
}

/// PTY session wrapping a subprocess
#[allow(clippy::arc_with_non_send_sync)]
pub struct PtySession {
    /// PTY system
    pty_system: NativePtySystem,
    /// Writer handle
    writer: Arc<RwLock<Box<dyn Write + Send>>>,
    /// Reader handle
    reader: Arc<RwLock<Box<dyn Read + Send>>>,
    /// Configuration
    config: PtyConfig,
}

impl PtySession {
    /// Create and start new PTY session
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(config: PtyConfig) -> Result<Self> {
        let pty_system = NativePtySystem::default();

        // Create PTY pair
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to create PTY")?;

        // Get reader and writer
        let reader = pty_pair
            .master
            .try_clone_reader()
            .context("Failed to clone PTY reader")?;
        let writer = pty_pair
            .master
            .take_writer()
            .context("Failed to take PTY writer")?;

        // Build command
        let mut cmd = CommandBuilder::new(&config.command);
        for arg in &config.args {
            cmd.arg(arg);
        }

        // Spawn child process
        let _child = pty_pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn command")?;

        Ok(Self {
            pty_system,
            writer: Arc::new(RwLock::new(writer)),
            reader: Arc::new(RwLock::new(reader)),
            config,
        })
    }

    /// Read output from PTY (non-blocking)
    pub async fn read(&self) -> Result<Option<PtyOutput>> {
        let mut reader = self.reader.write().await;
        let mut buffer = vec![0u8; 4096];

        match reader.read(&mut buffer) {
            Ok(0) => Ok(None), // EOF
            Ok(n) => {
                buffer.truncate(n);
                Ok(Some(PtyOutput {
                    data: buffer,
                    is_stderr: false,
                }))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Write input to PTY
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    /// Resize PTY
    pub async fn resize(&self, _cols: u16, _rows: u16) -> Result<()> {
        // Note: portable_pty doesn't expose resize directly on the master
        // This would need additional implementation
        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &PtyConfig {
        &self.config
    }
}
