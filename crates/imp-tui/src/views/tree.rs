use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// A flattened tree node for display.
#[derive(Debug, Clone)]
pub struct FlatTreeNode {
    pub id: String,
    pub depth: usize,
    pub summary: String,
    pub is_user: bool,
    pub is_tool: bool,
    pub has_children: bool,
    pub is_last_child: bool,
}

/// Filter mode for the tree view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeFilterMode {
    All,
    NoTools,
    UserOnly,
}

impl TreeFilterMode {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::NoTools,
            Self::NoTools => Self::UserOnly,
            Self::UserOnly => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NoTools => "no-tools",
            Self::UserOnly => "user-only",
        }
    }
}

/// State for the tree view overlay.
#[derive(Debug, Clone)]
pub struct TreeViewState {
    pub nodes: Vec<FlatTreeNode>,
    pub selected: usize,
    pub filter_mode: TreeFilterMode,
    pub current_id: Option<String>,
}

impl TreeViewState {
    pub fn new(nodes: Vec<FlatTreeNode>, current_id: Option<String>) -> Self {
        let selected = current_id
            .as_deref()
            .and_then(|id| nodes.iter().position(|n| n.id == id))
            .unwrap_or(0);
        Self {
            nodes,
            selected,
            filter_mode: TreeFilterMode::All,
            current_id,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_nodes().len() {
            self.selected += 1;
        }
    }

    pub fn selected_id(&self) -> Option<&str> {
        let filtered = self.filtered_nodes();
        filtered.get(self.selected).map(|n| n.id.as_str())
    }

    pub fn cycle_filter(&mut self) {
        self.filter_mode = self.filter_mode.next();
        self.selected = self.selected.min(self.filtered_nodes().len().saturating_sub(1));
    }

    fn filtered_nodes(&self) -> Vec<&FlatTreeNode> {
        self.nodes
            .iter()
            .filter(|n| match self.filter_mode {
                TreeFilterMode::All => true,
                TreeFilterMode::NoTools => !n.is_tool,
                TreeFilterMode::UserOnly => n.is_user,
            })
            .collect()
    }
}

/// Flatten a session tree into displayable nodes.
pub fn flatten_tree(tree: &[imp_core::session::TreeNode], depth: usize) -> Vec<FlatTreeNode> {
    let mut result = Vec::new();
    let len = tree.len();

    for (i, node) in tree.iter().enumerate() {
        match &node.entry {
            imp_core::session::SessionEntry::Message { id, message, .. } => {
                let text = extract_text(message);
                let truncated = if text.len() > 60 {
                    format!("{}…", &text[..57])
                } else {
                    text
                };
                let is_user = message.is_user();
                let is_tool = message.is_tool_result();
                result.push(FlatTreeNode {
                    id: id.clone(),
                    depth,
                    summary: truncated,
                    is_user,
                    is_tool,
                    has_children: !node.children.is_empty(),
                    is_last_child: i == len - 1,
                });
                result.extend(flatten_tree(&node.children, depth + 1));
            }
            imp_core::session::SessionEntry::Compaction { id, summary, .. } => {
                result.push(FlatTreeNode {
                    id: id.clone(),
                    depth,
                    summary: format!("[compaction: {}]", truncate(summary, 40)),
                    is_user: false,
                    is_tool: false,
                    has_children: !node.children.is_empty(),
                    is_last_child: i == len - 1,
                });
                result.extend(flatten_tree(&node.children, depth + 1));
            }
            _ => {}
        }
    }
    result
}

/// Session tree navigator overlay.
pub struct TreeView<'a> {
    state: &'a TreeViewState,
    theme: &'a Theme,
}

impl<'a> TreeView<'a> {
    pub fn new(state: &'a TreeViewState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for TreeView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        // Clear area and draw border
        Clear.render(area, buf);
        let block = Block::default()
            .title(format!(" Session Tree [{}] ", self.state.filter_mode.label()))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let filtered = self.state.nodes
            .iter()
            .filter(|n| match self.state.filter_mode {
                TreeFilterMode::All => true,
                TreeFilterMode::NoTools => !n.is_tool,
                TreeFilterMode::UserOnly => n.is_user,
            })
            .collect::<Vec<_>>();

        // Scroll to keep selected visible
        let visible_height = inner.height as usize;
        let scroll = if self.state.selected >= visible_height {
            self.state.selected - visible_height + 1
        } else {
            0
        };

        for (i, node) in filtered.iter().skip(scroll).enumerate() {
            if i >= visible_height {
                break;
            }

            let y = inner.y + i as u16;
            let is_selected = scroll + i == self.state.selected;
            let is_current = self.state.current_id.as_deref() == Some(&node.id);

            // Build tree prefix
            let indent = "  ".repeat(node.depth);
            let branch = if node.depth == 0 {
                ""
            } else if node.is_last_child {
                "└─ "
            } else {
                "├─ "
            };

            let marker = if is_current { "● " } else { "  " };
            let role_indicator = if node.is_user {
                "U"
            } else if node.is_tool {
                "T"
            } else {
                "A"
            };

            let style = if is_selected {
                self.theme.selected_style()
            } else if is_current {
                self.theme.accent_style()
            } else if node.is_tool {
                self.theme.muted_style()
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(marker.to_string(), self.theme.accent_style()),
                Span::styled(indent, self.theme.muted_style()),
                Span::styled(branch.to_string(), self.theme.muted_style()),
                Span::styled(
                    format!("[{role_indicator}] "),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled(node.summary.clone(), style),
            ]);

            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

fn extract_text(msg: &imp_llm::Message) -> String {
    let blocks = match msg {
        imp_llm::Message::User(u) => &u.content,
        imp_llm::Message::Assistant(a) => &a.content,
        imp_llm::Message::ToolResult(t) => &t.content,
    };
    blocks
        .iter()
        .find_map(|b| match b {
            imp_llm::ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
