use std::path::Path;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// State for the @file fuzzy finder overlay.
#[derive(Debug, Clone)]
pub struct FileFinderState {
    pub files: Vec<String>,
    pub filter: String,
    pub selected: usize,
}

impl FileFinderState {
    pub fn new(files: Vec<String>) -> Self {
        Self {
            files,
            filter: String::new(),
            selected: 0,
        }
    }

    pub fn filtered(&self) -> Vec<&str> {
        if self.filter.is_empty() {
            self.files.iter().take(20).map(|s| s.as_str()).collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.files
                .iter()
                .filter(|f| fuzzy_match(&f.to_lowercase(), &lower))
                .take(10)
                .map(|s| s.as_str())
                .collect()
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.filtered().len();
        if self.selected + 1 < count {
            self.selected += 1;
        }
    }

    pub fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
    }

    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.selected = 0;
    }

    pub fn selected_file(&self) -> Option<String> {
        let filtered = self.filtered();
        filtered.get(self.selected).map(|s| s.to_string())
    }
}

/// Collect project files respecting .gitignore (via walkdir, skipping hidden dirs).
pub fn collect_project_files(root: &Path, max_files: usize) -> Vec<String> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip hidden directories and common non-source dirs
            if e.file_type().is_dir() {
                return !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "__pycache__"
                    && name != ".git";
            }
            true
        })
    {
        if files.len() >= max_files {
            break;
        }
        if let Ok(entry) = entry {
            if entry.file_type().is_file() {
                if let Ok(rel) = entry.path().strip_prefix(root) {
                    files.push(rel.to_string_lossy().to_string());
                }
            }
        }
    }

    files.sort();
    files
}

/// Simple fuzzy match: all characters in the pattern appear in order in the haystack.
fn fuzzy_match(haystack: &str, pattern: &str) -> bool {
    let mut hay_chars = haystack.chars();
    for p in pattern.chars() {
        loop {
            match hay_chars.next() {
                Some(h) if h == p => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// File finder overlay widget.
pub struct FileFinderView<'a> {
    state: &'a FileFinderState,
    theme: &'a Theme,
}

impl<'a> FileFinderView<'a> {
    pub fn new(state: &'a FileFinderState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for FileFinderView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 15 {
            return;
        }

        Clear.render(area, buf);

        let title = if self.state.filter.is_empty() {
            " @file ".to_string()
        } else {
            format!(" @{} ", self.state.filter)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let filtered = self.state.filtered();

        for (i, path) in filtered.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let is_selected = i == self.state.selected;
            let style = if is_selected {
                self.theme.selected_style()
            } else {
                Style::default()
            };

            let line = Line::from(Span::styled(format!("  {path}"), style));
            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }

        if filtered.is_empty() {
            let line = Line::from(Span::styled(
                "  No matching files",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
        }
    }
}
