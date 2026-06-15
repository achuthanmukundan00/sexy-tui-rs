use crate::tui::Component;

/// A settings item with values to cycle through.
#[derive(Clone)]
pub struct SettingItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub current_value: String,
    pub values: Vec<String>,
}

/// SettingsList theme.
pub struct SettingsListTheme {
    pub label: Box<dyn Fn(&str, bool) -> String>,
    pub value: Box<dyn Fn(&str, bool) -> String>,
    pub description: Box<dyn Fn(&str) -> String>,
    pub cursor: String,
    pub hint: Box<dyn Fn(&str) -> String>,
}

/// Settings panel widget.
pub struct SettingsList {
    items: Vec<SettingItem>,
    selected: usize,
    max_visible: usize,
    on_change: Option<Box<dyn Fn(&str, &str)>>,
}

impl SettingsList {
    pub fn new(
        items: Vec<SettingItem>,
        max_visible: usize,
        _theme: SettingsListTheme,
        on_change: Box<dyn Fn(&str, &str)>,
    ) -> Self {
        SettingsList { items, selected: 0, max_visible,
            on_change: Some(on_change) }
    }

    pub fn update_value(&mut self, id: &str, value: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.current_value = value.to_string();
        }
    }
}

impl Component for SettingsList {
    fn render(&self, width: u16) -> Vec<String> {
        let end = (self.selected + self.max_visible).min(self.items.len());
        self.items[self.selected..end].iter().enumerate().map(|(i, item)| {
            let prefix = if i == 0 { "❯ " } else { "  " };
            crate::utils::truncate_to_width(
                &format!("{}{}: {}", prefix, item.label, item.current_value),
                width as usize, None)
        }).collect()
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};
        if matches_key(data, Key::up) && self.selected > 0 {
            self.selected -= 1;
        } else if matches_key(data, Key::down) && self.selected + 1 < self.items.len() {
            self.selected += 1;
        } else if matches_key(data, Key::enter) || matches_key(data, " ") {
            let item = &self.items[self.selected];
            if item.values.len() > 1 {
                let idx = item.values.iter().position(|v| v == &item.current_value)
                    .unwrap_or(0);
                let next = item.values[(idx + 1) % item.values.len()].clone();
                if let Some(ref cb) = self.on_change {
                    cb(&item.id, &next);
                }
            }
        }
    }

    fn invalidate(&mut self) {}
}
