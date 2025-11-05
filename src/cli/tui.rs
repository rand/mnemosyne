//! TUI wrapper command (deprecated)

use mnemosyne_core::{error::Result, icons};

/// Handle TUI wrapper command (deprecated)
pub async fn handle() -> Result<()> {
    // TUI wrapper mode is deprecated due to TUI-in-TUI conflicts
    eprintln!();
    eprintln!(
        "{}  DEPRECATED: 'mnemosyne tui' is no longer supported",
        icons::status::warning()
    );
    eprintln!();
    eprintln!("   The PTY wrapper mode has been removed due to terminal conflicts");
    eprintln!("   when wrapping Claude Code's TUI interface.");
    eprintln!();
    eprintln!("   {} New Architecture: Composable Tools", icons::data::folder());
    eprintln!();
    eprintln!("   Instead of wrapping Claude Code, Mnemosyne now provides");
    eprintln!("   standalone tools that work alongside it:");
    eprintln!();
    eprintln!("   1.  Edit Context:");
    eprintln!("      mnemosyne-ics context.md");
    eprintln!("      (Full-featured context editor with semantic highlighting)");
    eprintln!();
    eprintln!("   2.  Chat with Claude:");
    eprintln!("      claude");
    eprintln!("      (Memory integration happens automatically via MCP)");
    eprintln!();
    eprintln!("   3.  Monitor Activity:");
    eprintln!("      mnemosyne dash");
    eprintln!("      (Real-time dashboard - coming soon)");
    eprintln!();
    eprintln!(
        "   {} Tip: Use tmux/screen to see all tools at once:",
        icons::system::lightbulb()
    );
    eprintln!("      tmux split-window -h 'mnemosyne-ics context.md'");
    eprintln!("      tmux split-window -v 'mnemosyne dash'");
    eprintln!("      claude");
    eprintln!();
    eprintln!("   {} Migration Guide:", icons::data::folder());
    eprintln!("      https://github.com/rand/mnemosyne/blob/main/docs/MIGRATION.md");
    eprintln!();

    std::process::exit(1);
}
