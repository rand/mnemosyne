# Icon System

Mnemosyne uses **Nerd Font icons** (Font Awesome glyphs) to provide a polished, professional CLI experience with automatic fallback to ASCII alternatives for terminals without Nerd Font support.

## Overview

The icon system (`src/icons.rs`) provides:
- **20+ curated icons** organized into semantic categories
- **Automatic detection** of Nerd Font support
- **Graceful fallback** to colored ASCII alternatives
- **Zero runtime dependencies** - just unicode literals
- **Consistent color coding** for visual clarity

## Icon Categories

### Status Icons
Used for operation results and health indicators:

| Icon | Nerd Font | ASCII | Usage |
|------|-----------|-------|-------|
| Success | ✓ (green) | ✓ (green) | Successful operations, passing checks |
| Error | ✗ (red) | ✗ (red) | Failures, errors, critical issues |
| Warning | ⚠ (yellow) | ! (yellow) | Warnings, deprecations, non-critical issues |
| Info | ℹ (cyan) | i (cyan) | Information, tips, general messages |
| Ready | ★ (magenta) | * (magenta) | Completion, readiness indicators |

### Action Icons
Represent actions and operations:

| Icon | Nerd Font | ASCII | Usage |
|------|-----------|-------|-------|
| Target | ◎ (yellow) | ◎ (yellow) | Goals, objectives, targets |
| Search | 🔍 (cyan) | ? (cyan) | Search, find, query operations |
| Edit | ✏ (blue) | ~ (blue) | Edit, modify, write operations |
| Save | 💾 (magenta) | ↓ (magenta) | Save, persist, store operations |
| Link | 🔗 (cyan) | ∞ (cyan) | Links, connections, relationships |
| Build | 🔧 (yellow) | § (yellow) | Build, compile, construct |
| Launch | 🚀 (green) | » (green) | Launch, start, execute |
| Sync | ↻ (cyan) | ↻ (cyan) | Sync, refresh, update |
| Lightning | ⚡ (yellow) | ↯ (yellow) | Fast operations, performance |

### Data Icons
Represent data and information:

| Icon | Nerd Font | ASCII | Usage |
|------|-----------|-------|-------|
| Chart | 📊 (blue) | ▊ (blue) | Statistics, analytics, charts |
| Database | 💽 (magenta) | ◘ (magenta) | Database, storage, persistence |
| Folder | 📁 (yellow) | □ (yellow) | Files, directories, organization |
| Trending | 📈 (green) | ↗ (green) | Growth, trends, improvements |
| Brain | 🧠 (magenta) | ◉ (magenta) | Memory, intelligence, AI |

### System Icons
System and configuration indicators:

| Icon | Nerd Font | ASCII | Usage |
|------|-----------|-------|-------|
| Gear | ⚙ (gray) | ⚙ (gray) | Settings, configuration, system |
| Lightbulb | 💡 (yellow) | ○ (yellow) | Ideas, tips, suggestions |
| Palette | 🎨 (magenta) | ◆ (magenta) | Art, design, UI customization |
| Clock | 🕐 (cyan) | ◷ (cyan) | Time, duration, timestamps |
| Star | ⭐ (yellow) | ★ (yellow) | Important, favorites, highlights |

## Detection & Configuration

### Automatic Detection

The system automatically detects Nerd Font support by checking:

1. **Environment variables**:
   - `NERD_FONTS=1` - Force enable Nerd Fonts
   - `NERD_FONTS_DISABLED=1` - Force disable Nerd Fonts

2. **Terminal detection**:
   - iTerm2, WezTerm, Alacritty, Kitty - Auto-enabled
   - Other terminals - ASCII fallback (conservative default)

### Manual Configuration

**Enable Nerd Fonts**:
```bash
export NERD_FONTS=1
```

**Disable Nerd Fonts** (force ASCII):
```bash
export NERD_FONTS_DISABLED=1
```

**Add to your shell profile** (~/.zshrc, ~/.bashrc):
```bash
# Enable Nerd Font icons for mnemosyne
export NERD_FONTS=1
```

## Font Installation

### macOS (Homebrew)
```bash
brew install --cask font-jetbrains-mono-nerd-font
```

### Arch Linux
```bash
sudo pacman -S ttf-jetbrains-mono-nerd
```

### Ubuntu/Debian
```bash
# Install via script
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/install_manual.sh)"

# Or download manually
mkdir -p ~/.local/share/fonts
cd ~/.local/share/fonts
curl -fLO https://github.com/ryanoasis/nerd-fonts/releases/download/v3.0.2/JetBrainsMono.zip
unzip JetBrainsMono.zip
rm JetBrainsMono.zip
fc-cache -fv
```

### Windows
Download from [Nerd Fonts Releases](https://github.com/ryanoasis/nerd-fonts/releases) and install JetBrainsMono Nerd Font.

### Configure Your Terminal

After installing the font, configure your terminal to use it:
- **iTerm2**: Preferences → Profiles → Text → Font → JetBrainsMono Nerd Font
- **Alacritty**: Edit `~/.config/alacritty/alacritty.yml`:
  ```yaml
  font:
    normal:
      family: JetBrainsMono Nerd Font
  ```
- **Kitty**: Edit `~/.config/kitty/kitty.conf`:
  ```
  font_family JetBrainsMono Nerd Font
  ```

## Usage in Code

### Rust

```rust
use mnemosyne_core::icons;

// Status indicators
println!("{} Operation successful", icons::status::success());
println!("{} Error occurred", icons::status::error());
println!("{} Warning: deprecated feature", icons::status::warning());

// Actions
println!("{} Searching for memories...", icons::action::search());
println!("{} Saving to database...", icons::action::save());

// Data
println!("{} Database statistics:", icons::data::database());
println!("{} Chart data:", icons::data::chart());

// System
println!("{} Configuration loaded", icons::system::gear());
println!("{} Tip: Use --verbose for more details", icons::system::lightbulb());
```

### Shell Scripts

For shell scripts, use ASCII alternatives:

```bash
# Good symbols that work everywhere
echo "[✓] Success"
echo "[✗] Error"
echo "[!] Warning"
echo "[*] Info"
echo "[+] Added"
echo "[-] Removed"
echo "[~] Changed"
echo "[?] Searching"
echo "[#] Building"
```

## Color Scheme

Icons use semantic color coding:
- **Green** (32): Success, completion, growth
- **Red** (31): Errors, failures, critical issues
- **Yellow** (33): Warnings, cautions, highlights
- **Cyan** (36): Information, search, links
- **Magenta** (35): Important, special, memory-related
- **Blue** (34): Data, analysis, edit operations
- **Gray** (90): System, background operations

ANSI color format: `\x1b[<code>m<text>\x1b[0m`

## Testing

Run the Nerd Font rendering test:

```bash
cargo test test_font_awesome_glyphs -- --nocapture
```

This will display all icons and provide installation instructions if needed.

## Design Principles

1. **Conservative detection**: Default to ASCII to prevent rendering issues
2. **Semantic organization**: Icons grouped by purpose, not appearance
3. **Consistent colors**: Predictable color coding across all icons
4. **Font Awesome preference**: Maximum compatibility across Nerd Font variants
5. **Zero runtime cost**: Icons are compile-time constants

## Troubleshooting

### Icons show as boxes/squares

**Cause**: Terminal doesn't have Nerd Fonts installed or isn't detecting them.

**Solutions**:
1. Install a Nerd Font (see Font Installation above)
2. Configure your terminal to use the Nerd Font
3. Force ASCII mode: `export NERD_FONTS_DISABLED=1`

### Icons work in some terminals but not others

**Cause**: Inconsistent font configuration across terminals.

**Solution**: Ensure all terminals use a Nerd Font in their settings.

### Wrong icons displayed

**Cause**: Different Nerd Font variant or version.

**Solution**: Install JetBrainsMono Nerd Font specifically (our target font).

## Architecture Notes

- **Location**: `src/icons.rs` (library crate)
- **Initialization**: Lazy via `OnceLock` (once per process)
- **Storage**: Two static `IconSet` constants (Nerd + ASCII)
- **Detection**: Runtime check cached globally
- **Size**: ~300 lines, zero dependencies beyond stdlib

## References

- [Nerd Fonts Project](https://www.nerdfonts.com/)
- [Font Awesome Cheat Sheet](https://fontawesome.com/cheatsheet)
- [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code)
- [Unicode Private Use Area](https://en.wikipedia.org/wiki/Private_Use_Areas)
