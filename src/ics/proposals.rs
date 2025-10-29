//! Change proposal system for ICS
//!
//! Allows agents to propose changes that users can review, accept, or reject.
//! Proposals are shown in a dedicated panel with diff view.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use std::time::SystemTime;

/// Change proposal from an agent
#[derive(Debug, Clone)]
pub struct ChangeProposal {
    /// Unique proposal ID
    pub id: String,
    /// Agent that proposed the change
    pub agent: String,
    /// Proposal description
    pub description: String,
    /// Original text (before change)
    pub original: String,
    /// Proposed text (after change)
    pub proposed: String,
    /// Line range affected (start, end)
    pub line_range: (usize, usize),
    /// When proposal was created
    pub created_at: SystemTime,
    /// Current status
    pub status: ProposalStatus,
    /// Rationale for the change
    pub rationale: String,
}

/// Proposal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Pending review
    Pending,
    /// Accepted by user
    Accepted,
    /// Rejected by user
    Rejected,
    /// Applied to document
    Applied,
}

impl ProposalStatus {
    /// Get color for status
    pub fn color(&self) -> Color {
        match self {
            ProposalStatus::Pending => Color::Rgb(200, 180, 120),
            ProposalStatus::Accepted => Color::Rgb(180, 200, 180),
            ProposalStatus::Rejected => Color::Rgb(200, 140, 140),
            ProposalStatus::Applied => Color::Rgb(160, 180, 200),
        }
    }

    /// Get icon for status
    pub fn icon(&self) -> &'static str {
        match self {
            ProposalStatus::Pending => "◐",
            ProposalStatus::Accepted => "✓",
            ProposalStatus::Rejected => "✗",
            ProposalStatus::Applied => "●",
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ProposalStatus::Pending => "Pending",
            ProposalStatus::Accepted => "Accepted",
            ProposalStatus::Rejected => "Rejected",
            ProposalStatus::Applied => "Applied",
        }
    }
}

/// Proposals panel state
#[derive(Debug, Clone)]
pub struct ProposalsPanelState {
    /// List state for selection
    list_state: ListState,
    /// Whether panel is visible
    visible: bool,
    /// Whether to show proposal details
    show_details: bool,
    /// Filter by status
    status_filter: Option<ProposalStatus>,
}

impl Default for ProposalsPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProposalsPanelState {
    /// Create new proposals panel state
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            visible: false,
            show_details: false,
            status_filter: Some(ProposalStatus::Pending), // Default to pending only
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Toggle details view
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Set status filter
    pub fn set_status_filter(&mut self, filter: Option<ProposalStatus>) {
        self.status_filter = filter;
    }

    /// Get status filter
    pub fn status_filter(&self) -> Option<ProposalStatus> {
        self.status_filter
    }

    /// Select next proposal
    pub fn select_next(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(count - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Select previous proposal
    pub fn select_previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Get selected index
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }
}

/// Proposals panel widget
pub struct ProposalsPanel<'a> {
    /// Proposals to display
    proposals: &'a [ChangeProposal],
    /// Selected proposal for detail view
    selected_proposal: Option<&'a ChangeProposal>,
}

impl<'a> ProposalsPanel<'a> {
    /// Create new proposals panel
    pub fn new(proposals: &'a [ChangeProposal], selected: Option<&'a ChangeProposal>) -> Self {
        Self {
            proposals,
            selected_proposal: selected,
        }
    }
}

impl<'a> StatefulWidget for ProposalsPanel<'a> {
    type State = ProposalsPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        // Filter by status if set
        let filtered_proposals: Vec<&ChangeProposal> = if let Some(filter) = state.status_filter {
            self.proposals
                .iter()
                .filter(|p| p.status == filter)
                .collect()
        } else {
            self.proposals.iter().collect()
        };

        // Count by status
        let pending_count = self
            .proposals
            .iter()
            .filter(|p| p.status == ProposalStatus::Pending)
            .count();

        let title = format!(" Change Proposals ({} pending) ", pending_count);

        // Split into list and detail if showing details
        if state.show_details && self.selected_proposal.is_some() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40), // List
                    Constraint::Percentage(60), // Details
                ])
                .split(area);

            self.render_list(chunks[0], buf, state, &filtered_proposals, &title);
            if let Some(proposal) = self.selected_proposal {
                self.render_details(chunks[1], buf, proposal);
            }
        } else {
            self.render_list(area, buf, state, &filtered_proposals, &title);
        }
    }
}

impl<'a> ProposalsPanel<'a> {
    /// Render proposal list
    fn render_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut ProposalsPanelState,
        proposals: &[&ChangeProposal],
        title: &str,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        // Show empty state if no proposals
        if proposals.is_empty() {
            let inner = block.inner(area);
            block.render(area, buf);

            let empty_msg = if state.status_filter.is_some() {
                "No proposals with this status"
            } else if self.proposals.is_empty() {
                "No pending proposals"
            } else {
                "No results"
            };

            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    empty_msg,
                    Style::default().fg(Color::Rgb(140, 140, 160)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Agent change proposals will appear here",
                    Style::default().fg(Color::Rgb(120, 120, 140)),
                )),
            ];

            let paragraph = Paragraph::new(empty_text).alignment(ratatui::layout::Alignment::Center);
            paragraph.render(inner, buf);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = proposals
            .iter()
            .map(|proposal| {
                let icon = proposal.status.icon();
                let color = proposal.status.color();

                let agent_name = if proposal.agent.starts_with("agent:") {
                    &proposal.agent[6..]
                } else {
                    &proposal.agent
                };

                let line = Line::from(vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(agent_name, Style::default().fg(Color::Rgb(180, 160, 200))),
                    Span::raw(" @ Ln "),
                    Span::styled(
                        (proposal.line_range.0 + 1).to_string(),
                        Style::default().fg(Color::Rgb(160, 160, 160)),
                    ),
                    Span::raw(": "),
                    Span::raw(&proposal.description),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }

    /// Render proposal details
    fn render_details(&self, area: Rect, buf: &mut Buffer, proposal: &ChangeProposal) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Proposal Details ")
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        let inner = block.inner(area);
        block.render(area, buf);

        // Format details
        let lines = vec![
            Line::from(vec![
                Span::styled("Agent: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&proposal.agent),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(proposal.status.name(), Style::default().fg(proposal.status.color())),
            ]),
            Line::from(""),
            Line::from(Span::styled("Rationale:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(proposal.rationale.as_str()),
            Line::from(""),
            Line::from(Span::styled("Diff:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(vec![
                Span::styled("- ", Style::default().fg(Color::Rgb(200, 140, 140))),
                Span::raw(&proposal.original),
            ]),
            Line::from(vec![
                Span::styled("+ ", Style::default().fg(Color::Rgb(180, 200, 180))),
                Span::raw(&proposal.proposed),
            ]),
            Line::from(""),
            Line::from("Press 'a' to accept, 'r' to reject"),
        ];

        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposals_panel_state() {
        let mut state = ProposalsPanelState::new();
        assert!(!state.is_visible());
        assert_eq!(state.status_filter(), Some(ProposalStatus::Pending));

        state.toggle();
        assert!(state.is_visible());

        state.hide();
        assert!(!state.is_visible());

        state.show();
        assert!(state.is_visible());
    }

    #[test]
    fn test_proposal_selection() {
        let mut state = ProposalsPanelState::new();
        assert_eq!(state.selected(), None);

        state.select_next(5);
        assert_eq!(state.selected(), Some(0));

        state.select_next(5);
        assert_eq!(state.selected(), Some(1));

        state.select_previous();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_status_filter() {
        let mut state = ProposalsPanelState::new();
        assert_eq!(state.status_filter(), Some(ProposalStatus::Pending));

        state.set_status_filter(Some(ProposalStatus::Accepted));
        assert_eq!(state.status_filter(), Some(ProposalStatus::Accepted));

        state.set_status_filter(None);
        assert_eq!(state.status_filter(), None);
    }

    #[test]
    fn test_proposal_status_properties() {
        for status in [
            ProposalStatus::Pending,
            ProposalStatus::Accepted,
            ProposalStatus::Rejected,
            ProposalStatus::Applied,
        ] {
            let _ = status.color();
            assert!(!status.icon().is_empty());
            assert!(!status.name().is_empty());
        }
    }

    #[test]
    fn test_change_proposal() {
        let proposal = ChangeProposal {
            id: "prop-1".to_string(),
            agent: "agent:semantic".to_string(),
            description: "Fix typo".to_string(),
            original: "teh".to_string(),
            proposed: "the".to_string(),
            line_range: (5, 5),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Common typo detected".to_string(),
        };

        assert_eq!(proposal.id, "prop-1");
        assert_eq!(proposal.status, ProposalStatus::Pending);
    }

    #[test]
    fn test_toggle_details() {
        let mut state = ProposalsPanelState::new();
        assert!(!state.show_details);

        state.toggle_details();
        assert!(state.show_details);

        state.toggle_details();
        assert!(!state.show_details);
    }
}
