//! Icon system with Nerd Font glyphs (Font Awesome) and ASCII fallbacks
//!
//! Uses Font Awesome icons from Nerd Fonts for maximum compatibility.
//! Automatically detects terminal capabilities and falls back to ASCII.
//!
//! # Usage
//!
//! ```rust
//! use mnemosyne_core::icons;
//!
//! println!("{} Success!", icons::status::success());
//! println!("{} Searching...", icons::action::search());
//! ```
//!
//! # Environment Variables
//!
//! - `NERD_FONTS=1`: Force Nerd Font mode
//! - `NERD_FONTS_DISABLED=1`: Force ASCII mode
//!
//! # Font Installation
//!
//! Install JetBrains Mono Nerd Font for best results:
//! ```bash
//! brew install --cask font-jetbrains-mono-nerd-font
//! ```

use std::sync::OnceLock;

/// Icon set with colored glyphs
#[derive(Debug, Clone)]
pub struct IconSet {
    // Status
    pub success: &'static str,
    pub error: &'static str,
    pub warning: &'static str,
    pub info: &'static str,
    pub ready: &'static str,

    // Actions
    pub target: &'static str,
    pub search: &'static str,
    pub edit: &'static str,
    pub save: &'static str,
    pub link: &'static str,
    pub build: &'static str,
    pub launch: &'static str,
    pub sync: &'static str,
    pub lightning: &'static str,

    // Data
    pub chart: &'static str,
    pub database: &'static str,
    pub folder: &'static str,
    pub trending: &'static str,
    pub brain: &'static str,

    // System
    pub gear: &'static str,
    pub lightbulb: &'static str,
    pub palette: &'static str,
    pub clock: &'static str,
    pub star: &'static str,
}

/// Nerd Font icons (Font Awesome) with ANSI colors
///
/// All icons use Font Awesome from Nerd Fonts (Private Use Area U+F000-U+F2E0)
/// with the exception of `brain` which uses Material Design Icons for better visual.
const NERD_ICONS: IconSet = IconSet {
    // Status - Font Awesome
    success: "\x1b[32m\u{f00c}\x1b[0m",      // fa-check (green)
    error: "\x1b[31m\u{f00d}\x1b[0m",        // fa-times (red)
    warning: "\x1b[33m\u{f071}\x1b[0m",      // fa-exclamation-triangle (yellow)
    info: "\x1b[36m\u{f05a}\x1b[0m",         // fa-info-circle (cyan)
    ready: "\x1b[35m\u{f005}\x1b[0m",        // fa-star (magenta)

    // Actions - Font Awesome
    target: "\x1b[33m\u{f140}\x1b[0m",       // fa-bullseye (yellow)
    search: "\x1b[36m\u{f002}\x1b[0m",       // fa-search (cyan)
    edit: "\x1b[34m\u{f040}\x1b[0m",         // fa-pencil (blue)
    save: "\x1b[35m\u{f0c7}\x1b[0m",         // fa-floppy-o (magenta)
    link: "\x1b[36m\u{f0c1}\x1b[0m",         // fa-link (cyan)
    build: "\x1b[33m\u{f0ad}\x1b[0m",        // fa-wrench (yellow)
    launch: "\x1b[32m\u{f135}\x1b[0m",       // fa-rocket (green)
    sync: "\x1b[36m\u{f021}\x1b[0m",         // fa-refresh (cyan)
    lightning: "\x1b[33m\u{f0e7}\x1b[0m",    // fa-bolt (yellow)

    // Data - Font Awesome (except brain)
    chart: "\x1b[34m\u{f080}\x1b[0m",        // fa-bar-chart (blue)
    database: "\x1b[35m\u{f1c0}\x1b[0m",     // fa-database (magenta)
    folder: "\x1b[33m\u{f07b}\x1b[0m",       // fa-folder-open (yellow)
    trending: "\x1b[32m\u{f201}\x1b[0m",     // fa-line-chart (green)
    brain: "\x1b[35m\u{f5dc}\x1b[0m",        // md-brain (Material Design, magenta)

    // System - Font Awesome
    gear: "\x1b[90m\u{f013}\x1b[0m",         // fa-gear (gray)
    lightbulb: "\x1b[33m\u{f0eb}\x1b[0m",    // fa-lightbulb-o (yellow)
    palette: "\x1b[35m\u{f53f}\x1b[0m",      // fa-paint-brush (magenta)
    clock: "\x1b[36m\u{f017}\x1b[0m",        // fa-clock-o (cyan)
    star: "\x1b[33m\u{f005}\x1b[0m",         // fa-star (yellow)
};

/// ASCII fallback icons (colored, no special fonts needed)
///
/// Uses standard Unicode characters that render on all terminals.
const ASCII_ICONS: IconSet = IconSet {
    // Status
    success: "\x1b[32m✓\x1b[0m",             // Check mark (green)
    error: "\x1b[31m✗\x1b[0m",               // Ballot X (red)
    warning: "\x1b[33m!\x1b[0m",             // Exclamation (yellow)
    info: "\x1b[36mi\x1b[0m",                // Lowercase i (cyan)
    ready: "\x1b[35m*\x1b[0m",               // Asterisk (magenta)

    // Actions
    target: "\x1b[33m◎\x1b[0m",              // Bullseye (yellow)
    search: "\x1b[36m?\x1b[0m",              // Question mark (cyan)
    edit: "\x1b[34m~\x1b[0m",                // Tilde (blue)
    save: "\x1b[35m↓\x1b[0m",                // Downward arrow (magenta)
    link: "\x1b[36m∞\x1b[0m",                // Infinity (cyan)
    build: "\x1b[33m§\x1b[0m",               // Section sign (yellow)
    launch: "\x1b[32m»\x1b[0m",              // Right guillemet (green)
    sync: "\x1b[36m↻\x1b[0m",                // Clockwise arrow (cyan)
    lightning: "\x1b[33m↯\x1b[0m",           // Downward zigzag (yellow)

    // Data
    chart: "\x1b[34m▊\x1b[0m",               // Left 3/4 block (blue)
    database: "\x1b[35m◘\x1b[0m",            // Inverse bullet (magenta)
    folder: "\x1b[33m□\x1b[0m",              // White square (yellow)
    trending: "\x1b[32m↗\x1b[0m",            // Up-right arrow (green)
    brain: "\x1b[35m◉\x1b[0m",               // Fish eye (magenta)

    // System
    gear: "\x1b[90m⚙\x1b[0m",                // Gear (gray)
    lightbulb: "\x1b[33m○\x1b[0m",           // White circle (yellow)
    palette: "\x1b[35m◆\x1b[0m",             // Black diamond (magenta)
    clock: "\x1b[36m◷\x1b[0m",               // Circle 1/4 white (cyan)
    star: "\x1b[33m★\x1b[0m",                // Black star (yellow)
};

/// Global icon set (initialized once at runtime)
static ICONS: OnceLock<&'static IconSet> = OnceLock::new();

/// Detect if terminal supports Nerd Fonts
///
/// Checks environment variables and known terminal types.
/// Conservative default (ASCII) to prevent rendering issues.
fn supports_nerd_fonts() -> bool {
    // User override (set by install script or manually)
    if let Ok(val) = std::env::var("NERD_FONTS") {
        return val == "1" || val.eq_ignore_ascii_case("true");
    }

    // Explicit disable
    if std::env::var("NERD_FONTS_DISABLED").is_ok() {
        return false;
    }

    // Known terminals with good Nerd Font support
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        let term_lower = term_program.to_lowercase();
        if term_lower.contains("iterm")
            || term_lower.contains("wezterm")
            || term_lower.contains("alacritty")
            || term_lower.contains("kitty")
        {
            return true;
        }
    }

    // Check TERM variable for kitty/alacritty
    if let Ok(term) = std::env::var("TERM") {
        let term_lower = term.to_lowercase();
        if term_lower.contains("kitty") || term_lower.contains("alacritty") {
            return true;
        }
    }

    // Conservative default: use ASCII unless explicitly enabled
    // This prevents rendering issues on unsupported terminals
    false
}

/// Get the active icon set (initialized on first call)
///
/// Automatically detects Nerd Font support and returns appropriate set.
pub fn icons() -> &'static IconSet {
    ICONS.get_or_init(|| {
        if supports_nerd_fonts() {
            &NERD_ICONS
        } else {
            &ASCII_ICONS
        }
    })
}

/// Status icons (success, error, warning, info, ready)
pub mod status {
    use super::icons;

    /// Success/completion icon (green check or ✓)
    pub fn success() -> &'static str { icons().success }

    /// Error/failure icon (red X or ✗)
    pub fn error() -> &'static str { icons().error }

    /// Warning/alert icon (yellow triangle or !)
    pub fn warning() -> &'static str { icons().warning }

    /// Information icon (cyan circle-i or i)
    pub fn info() -> &'static str { icons().info }

    /// Ready/completion icon (magenta star or *)
    pub fn ready() -> &'static str { icons().ready }
}

/// Action icons (search, edit, save, launch, etc.)
pub mod action {
    use super::icons;

    /// Target/goal icon (yellow bullseye or ◎)
    pub fn target() -> &'static str { icons().target }

    /// Search/find icon (cyan magnifier or ?)
    pub fn search() -> &'static str { icons().search }

    /// Edit/write icon (blue pencil or ~)
    pub fn edit() -> &'static str { icons().edit }

    /// Save/persist icon (magenta floppy or ↓)
    pub fn save() -> &'static str { icons().save }

    /// Link/connection icon (cyan chain or ∞)
    pub fn link() -> &'static str { icons().link }

    /// Build/tools icon (yellow wrench or §)
    pub fn build() -> &'static str { icons().build }

    /// Launch/start icon (green rocket or »)
    pub fn launch() -> &'static str { icons().launch }

    /// Sync/refresh icon (cyan circular arrows or ↻)
    pub fn sync() -> &'static str { icons().sync }

    /// Fast/lightning icon (yellow bolt or ↯)
    pub fn lightning() -> &'static str { icons().lightning }
}

/// Data icons (charts, database, folders, etc.)
pub mod data {
    use super::icons;

    /// Chart/statistics icon (blue bars or ▊)
    pub fn chart() -> &'static str { icons().chart }

    /// Database icon (magenta cylinder or ◘)
    pub fn database() -> &'static str { icons().database }

    /// Folder/files icon (yellow folder or □)
    pub fn folder() -> &'static str { icons().folder }

    /// Trending/growth icon (green line chart or ↗)
    pub fn trending() -> &'static str { icons().trending }

    /// Brain/memory icon (magenta brain or ◉)
    pub fn brain() -> &'static str { icons().brain }
}

/// System icons (settings, ideas, time, etc.)
pub mod system {
    use super::icons;

    /// Settings/configuration icon (gray gear or ⚙)
    pub fn gear() -> &'static str { icons().gear }

    /// Ideas/tips icon (yellow lightbulb or ○)
    pub fn lightbulb() -> &'static str { icons().lightbulb }

    /// Art/design icon (magenta palette or ◆)
    pub fn palette() -> &'static str { icons().palette }

    /// Time/clock icon (cyan clock or ◷)
    pub fn clock() -> &'static str { icons().clock }

    /// Star/special icon (yellow star or ★)
    pub fn star() -> &'static str { icons().star }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_set_access() {
        let icons = icons();
        assert!(!icons.success.is_empty());
        assert!(!icons.error.is_empty());
        assert!(!icons.search.is_empty());
    }

    #[test]
    fn test_module_access() {
        assert!(!status::success().is_empty());
        assert!(!action::launch().is_empty());
        assert!(!data::chart().is_empty());
        assert!(!system::gear().is_empty());
    }

    #[test]
    fn test_nerd_font_detection() {
        // Just verify it doesn't panic
        let _ = supports_nerd_fonts();
    }

    #[test]
    fn test_env_override() {
        // Test NERD_FONTS=1 enables nerd fonts
        std::env::set_var("NERD_FONTS", "1");
        assert!(supports_nerd_fonts());
        std::env::remove_var("NERD_FONTS");

        // Test NERD_FONTS_DISABLED disables them
        std::env::set_var("NERD_FONTS_DISABLED", "1");
        assert!(!supports_nerd_fonts());
        std::env::remove_var("NERD_FONTS_DISABLED");
    }
}
