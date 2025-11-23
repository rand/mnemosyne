use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use std::time::{Duration, Instant};

/// Notification kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationKind {
    pub fn color(&self) -> Color {
        match self {
            NotificationKind::Info => Color::Blue,
            NotificationKind::Success => Color::Green,
            NotificationKind::Warning => Color::Yellow,
            NotificationKind::Error => Color::Red,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            NotificationKind::Info => "ℹ",
            NotificationKind::Success => "✓",
            NotificationKind::Warning => "⚠",
            NotificationKind::Error => "✗",
        }
    }
}

/// A single notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub kind: NotificationKind,
    pub timestamp: Instant,
    pub ttl: Duration,
}

/// Manages active notifications
#[derive(Debug)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    pub fn notify(&mut self, kind: NotificationKind, message: String) {
        self.notifications.push(Notification {
            message,
            kind,
            timestamp: Instant::now(),
            ttl: Duration::from_secs(5), // Default 5 seconds
        });
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.notifications
            .retain(|n| now.duration_since(n.timestamp) < n.ttl);
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &NotificationManager {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render notifications in top-right corner, stacked vertically
        let width = 40; // Fixed width for notifications
        let right_margin = 2;
        let top_margin = 1;

        let mut y = top_margin;

        // Render newest first (at the top)
        // We iterate in reverse order of addition if we append new ones to the end
        // But typically we want the newest one at the top or bottom?
        // Let's render them in order of the vector (oldest first) or newest first?
        // Standard toast behavior: usually stack up or down.
        // Let's go with: newest at the bottom of the stack? Or newest at top?
        // Let's iterate reversed so newest (end of vec) is at the top?
        // Actually, if I push to end, `rev()` gives me the newest first.

        for notification in self.notifications.iter().rev() {
            let available_height = area.height.saturating_sub(y);
            if available_height < 3 {
                break;
            }

            // Calculate height needed
            // Simple approximation: 3 lines (border + 1 line of text)
            // If text is long, it wraps.

            // Let's assume 3 lines for now for simplicity, or calculate text height?
            // Paragraph widget handles wrapping, but we need to give it a rect.
            // A fixed height of 3 is safe for short messages.
            let height = 3;

            let x = area.width.saturating_sub(width + right_margin);
            let notif_area = Rect::new(x, y, width, height);

            // Clear background to ensure text is readable over other widgets
            Clear.render(notif_area, buf);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(notification.kind.color()));

            let text = format!("{}  {}", notification.kind.icon(), notification.message);
            let p = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));

            p.render(notif_area, buf);

            y += height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager() {
        let mut manager = NotificationManager::new();
        manager.notify(NotificationKind::Info, "Test".to_string());
        assert_eq!(manager.notifications.len(), 1);
    }

    #[test]
    fn test_expiration() {
        let mut manager = NotificationManager::new();
        manager.notify(NotificationKind::Info, "Test".to_string());

        // Manually expire it
        manager.notifications[0].timestamp = Instant::now() - Duration::from_secs(10);

        manager.tick();
        assert_eq!(manager.notifications.len(), 0);
    }
}
