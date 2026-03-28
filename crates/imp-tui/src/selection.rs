use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectablePane {
    Chat,
    SidebarDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Chat,
    SidebarList,
    SidebarDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionState {
    pub pane: SelectablePane,
    pub anchor: SelectionPos,
    pub focus: SelectionPos,
}

impl SelectionState {
    pub fn new(pane: SelectablePane, anchor: SelectionPos, focus: SelectionPos) -> Self {
        Self {
            pane,
            anchor,
            focus,
        }
    }

    pub fn normalized(&self) -> (SelectionPos, SelectionPos) {
        if (self.anchor.line, self.anchor.col) <= (self.focus.line, self.focus.col) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextSurface {
    pub pane: SelectablePane,
    pub rect: Rect,
    pub lines: Vec<String>,
    pub top_line: usize,
}

impl TextSurface {
    pub fn new(pane: SelectablePane, rect: Rect, lines: Vec<String>, top_line: usize) -> Self {
        Self {
            pane,
            rect,
            lines,
            top_line,
        }
    }

    pub fn contains(&self, col: u16, row: u16) -> bool {
        self.rect.width > 0
            && self.rect.height > 0
            && col >= self.rect.x
            && col < self.rect.x + self.rect.width
            && row >= self.rect.y
            && row < self.rect.y + self.rect.height
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn default_pos(&self) -> SelectionPos {
        SelectionPos {
            line: self.top_line.min(self.lines.len().saturating_sub(1)),
            col: 0,
        }
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|s| s.chars().count()).unwrap_or(0)
    }

    pub fn clamp_pos(&self, mut pos: SelectionPos) -> SelectionPos {
        if self.lines.is_empty() {
            return SelectionPos { line: 0, col: 0 };
        }

        pos.line = pos.line.min(self.lines.len().saturating_sub(1));
        let len = self.line_len(pos.line);
        pos.col = if len == 0 { 0 } else { pos.col.min(len - 1) };
        pos
    }

    pub fn pos_from_screen_clamped(&self, col: u16, row: u16) -> SelectionPos {
        if self.lines.is_empty() {
            return SelectionPos { line: 0, col: 0 };
        }

        let clamped_row = row.clamp(
            self.rect.y,
            self.rect.y + self.rect.height.saturating_sub(1),
        );
        let visible_line = (clamped_row - self.rect.y) as usize;
        let line = (self.top_line + visible_line).min(self.lines.len().saturating_sub(1));

        let line_len = self.line_len(line);
        let relative_col = col.saturating_sub(self.rect.x) as usize;
        let bounded_col = if line_len == 0 {
            0
        } else {
            relative_col.min(line_len - 1)
        };

        SelectionPos {
            line,
            col: bounded_col,
        }
    }

    pub fn visible_row_for_line(&self, line: usize) -> Option<u16> {
        if line < self.top_line {
            return None;
        }
        let rel = line - self.top_line;
        if rel >= self.rect.height as usize {
            None
        } else {
            Some(self.rect.y + rel as u16)
        }
    }

    pub fn move_pos(&self, pos: SelectionPos, line_delta: isize, col_delta: isize) -> SelectionPos {
        if self.lines.is_empty() {
            return SelectionPos { line: 0, col: 0 };
        }

        let max_line = self.lines.len().saturating_sub(1) as isize;
        let next_line = (pos.line as isize + line_delta).clamp(0, max_line) as usize;
        let desired_col = if col_delta.is_negative() {
            pos.col.saturating_sub(col_delta.unsigned_abs())
        } else {
            pos.col.saturating_add(col_delta as usize)
        };

        self.clamp_pos(SelectionPos {
            line: next_line,
            col: desired_col,
        })
    }
}

pub fn extract_selected_text(surface: &TextSurface, selection: &SelectionState) -> Option<String> {
    if selection.pane != surface.pane || surface.lines.is_empty() {
        return None;
    }

    let (start, end) = selection.normalized();
    let start = surface.clamp_pos(start);
    let end = surface.clamp_pos(end);

    let mut out = String::new();
    for line_idx in start.line..=end.line {
        let line = surface.lines.get(line_idx)?;
        let chars: Vec<char> = line.chars().collect();

        let slice = if chars.is_empty() {
            String::new()
        } else if start.line == end.line {
            chars[start.col..=end.col].iter().collect()
        } else if line_idx == start.line {
            chars[start.col..].iter().collect()
        } else if line_idx == end.line {
            chars[..=end.col].iter().collect()
        } else {
            chars.iter().collect()
        };

        out.push_str(&slice);
        if line_idx != end.line {
            out.push('\n');
        }
    }

    Some(out)
}

pub struct SelectionOverlay<'a> {
    theme: &'a Theme,
    selection: Option<&'a SelectionState>,
    chat_surface: Option<&'a TextSurface>,
    sidebar_surface: Option<&'a TextSurface>,
}

impl<'a> SelectionOverlay<'a> {
    pub fn new(
        theme: &'a Theme,
        selection: Option<&'a SelectionState>,
        chat_surface: Option<&'a TextSurface>,
        sidebar_surface: Option<&'a TextSurface>,
    ) -> Self {
        Self {
            theme,
            selection,
            chat_surface,
            sidebar_surface,
        }
    }

    fn surface_for_selection(&self) -> Option<&TextSurface> {
        let selection = self.selection?;
        match selection.pane {
            SelectablePane::Chat => self.chat_surface,
            SelectablePane::SidebarDetail => self.sidebar_surface,
        }
    }
}

impl Widget for SelectionOverlay<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        let Some(selection) = self.selection else {
            return;
        };
        let Some(surface) = self.surface_for_selection() else {
            return;
        };
        if surface.lines.is_empty() {
            return;
        }

        let (start, end) = selection.normalized();
        let start = surface.clamp_pos(start);
        let end = surface.clamp_pos(end);
        let style = self.theme.selected_style();

        for line_idx in start.line..=end.line {
            let Some(row) = surface.visible_row_for_line(line_idx) else {
                continue;
            };
            let line_len = surface.line_len(line_idx);
            if line_len == 0 {
                continue;
            }

            let start_col = if line_idx == start.line { start.col } else { 0 };
            let end_col = if line_idx == end.line {
                end.col.min(line_len.saturating_sub(1))
            } else {
                line_len.saturating_sub(1)
            };

            for col in start_col..=end_col {
                let x = surface.rect.x + col as u16;
                if x >= surface.rect.x + surface.rect.width {
                    break;
                }
                if let Some(cell) = buf.cell_mut((x, row)) {
                    cell.set_style(style);
                }
            }
        }
    }
}
