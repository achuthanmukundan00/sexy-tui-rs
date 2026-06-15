use crate::tui::Component;
use crate::fuzzy::fuzzy_filter;

/// A selectable item.
#[derive(Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

/// Theme for SelectList.
pub struct SelectListTheme {
    pub selected_prefix: Box<dyn Fn(&str) -> String>,
    pub selected_text: Box<dyn Fn(&str) -> String>,
    pub description: Box<dyn Fn(&str) -> String>,
    pub scroll_info: Box<dyn Fn(&str) -> String>,
    pub no_match: Box<dyn Fn(&str) -> String>,
}

/// Interactive selection list widget.
pub struct SelectList {
    items: Vec<SelectItem>,
    filtered: Vec<usize>,
    selected: usize,
    filter: String,
    max_visible: usize,
    scroll_offset: usize,
}

impl SelectList {
    pub fn new(items: Vec<SelectItem>, max_visible: usize, _theme: SelectListTheme) -> Self {
        let filtered: Vec<usize> = (0..items.len()).collect();
        SelectList { items, filtered, selected: 0, filter: String::new(),
            max_visible, scroll_offset: 0 }
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        let filter_str = self.filter.clone();
        // Collect labels first to avoid borrowing self in closure
        let labels: Vec<String> = self.items.iter().map(|item| item.label.clone()).collect();
        let indices: Vec<usize> = (0..labels.len()).collect();
        self.filtered = fuzzy_filter(&indices, &filter_str, |i| labels[*i].clone());
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn selected_item(&self) -> Option<&SelectItem> {
        self.filtered.get(self.selected).map(|&i| &self.items[i])
    }
}

impl Component for SelectList {
    fn render(&self, width: u16) -> Vec<String> {
        let end = (self.scroll_offset + self.max_visible).min(self.filtered.len());
        let visible = &self.filtered[self.scroll_offset..end];

        let mut lines: Vec<String> = visible.iter().enumerate().map(|(i, &idx)| {
            let item = &self.items[idx];
            let prefix = if self.scroll_offset + i == self.selected { "❯ " } else { "  " };
            let line = if let Some(ref desc) = item.description {
                format!("{}{} — {}", prefix, item.label, desc)
            } else {
                format!("{}{}", prefix, item.label)
            };
            crate::utils::truncate_to_width(&line, width as usize, None)
        }).collect();

        if self.filtered.len() > self.max_visible {
            lines.push(format!("[{}/{}+]", self.selected + 1, self.filtered.len()));
        }
        lines
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};
        if matches_key(data, Key::up) && self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        } else if matches_key(data, Key::down) && self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + self.max_visible {
                self.scroll_offset = self.selected + 1 - self.max_visible;
            }
        }
    }

    fn invalidate(&mut self) {}
}
