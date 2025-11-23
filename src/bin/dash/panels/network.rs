//! Network Panel - P2P Network State
//!
//! Visualizes the Iroh P2P network:
//! - Connected peers
//! - Known nodes
//! - Recent network activity

use crate::colors::DashboardColors;
use mnemosyne_core::api::events::{Event, EventType};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub struct NetworkPanel {
    pub connected_peers: usize,
    pub known_nodes: Vec<String>,
}

impl NetworkPanel {
    pub fn new() -> Self {
        Self {
            connected_peers: 0,
            known_nodes: Vec::new(),
        }
    }

    pub fn update(&mut self, event: Event) {
        if let EventType::NetworkStateUpdate {
            connected_peers,
            known_nodes,
            ..
        } = event.event_type
        {
            self.connected_peers = connected_peers;
            self.known_nodes = known_nodes;
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();

        items.push(ListItem::new(Line::from(vec![
            Span::raw("Connected Peers: "),
            Span::styled(
                format!("{}", self.connected_peers),
                Style::default()
                    .fg(DashboardColors::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
        ])));

        items.push(ListItem::new(Line::from("Known Nodes:")));
        if self.known_nodes.is_empty() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                "  (none)",
                Style::default()
                    .fg(DashboardColors::SECONDARY)
                    .add_modifier(Modifier::ITALIC),
            )])));
        } else {
            for node in &self.known_nodes {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("  - "),
                    Span::styled(node, Style::default().fg(DashboardColors::SECONDARY)),
                ])));
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Network State")
                .border_style(Style::default().fg(DashboardColors::BORDER)),
        );

        frame.render_widget(list, area);
    }
}
