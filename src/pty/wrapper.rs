//! Claude Code wrapper with PTY interception

use super::{OutputParser, ParsedChunk, PtyConfig, PtySession};
use anyhow::Result;
use tokio::sync::mpsc;

/// Claude Code wrapper
pub struct ClaudeCodeWrapper {
    /// PTY session
    session: PtySession,
    /// Output parser
    parser: OutputParser,
    /// Output channel
    output_tx: mpsc::UnboundedSender<ParsedChunk>,
    /// Output receiver
    output_rx: Option<mpsc::UnboundedReceiver<ParsedChunk>>,
}

impl ClaudeCodeWrapper {
    /// Create new wrapper
    pub fn new(config: PtyConfig) -> Result<Self> {
        let session = PtySession::new(config)?;
        let parser = OutputParser::new();
        let (output_tx, output_rx) = mpsc::unbounded_channel();

        Ok(Self {
            session,
            parser,
            output_tx,
            output_rx: Some(output_rx),
        })
    }

    /// Take the output receiver (can only be called once)
    pub fn take_output_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<ParsedChunk>> {
        self.output_rx.take()
    }

    /// Send input to Claude Code
    pub async fn send_input(&self, data: &[u8]) -> Result<()> {
        self.session.write(data).await
    }

    /// Poll for output
    pub async fn poll_output(&mut self) -> Result<()> {
        if let Some(output) = self.session.read().await? {
            let chunks = self.parser.parse(&output.data);
            for chunk in chunks {
                let _ = self.output_tx.send(chunk);
            }
        }
        Ok(())
    }

    /// Resize terminal
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.session.resize(cols, rows).await
    }
}
