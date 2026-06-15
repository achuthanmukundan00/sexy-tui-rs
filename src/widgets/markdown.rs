use crate::tui::Component;

pub struct MarkdownTheme {
    pub heading: Box<dyn Fn(&str) -> String>,
    pub bold: Box<dyn Fn(&str) -> String>,
    pub code: Box<dyn Fn(&str) -> String>,
    pub code_block_border: Box<dyn Fn(&str) -> String>,
}

pub struct MarkdownOptions {
    pub padding_x: u16,
    pub padding_y: u16,
}

impl Default for MarkdownOptions {
    fn default() -> Self { MarkdownOptions { padding_x: 1, padding_y: 1 } }
}

/// Markdown renderer widget.
pub struct Markdown {
    content: String,
    padding_x: u16,
    padding_y: u16,
    theme: Option<MarkdownTheme>,
}

impl Markdown {
    pub fn new(content: &str, padding_x: u16, padding_y: u16, theme: Option<MarkdownTheme>) -> Self {
        Markdown { content: content.to_string(), padding_x, padding_y, theme }
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = text.to_string();
    }
}

impl Component for Markdown {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = width.saturating_sub(self.padding_x * 2);
        let spacer = " ".repeat(self.padding_x as usize);
        let mut lines = vec!["".to_string(); self.padding_y as usize];

        for line in self.content.lines() {
            let rendered = if let Some(ref theme) = self.theme {
                if line.starts_with("# ") {
                    (theme.heading)(line)
                } else if line.starts_with("```") {
                    (theme.code_block_border)(line)
                } else if line.starts_with('`') && line.ends_with('`') {
                    (theme.code)(line)
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            };

            for wrapped in crate::utils::wrap_text_with_ansi(&rendered, inner as usize) {
                lines.push(format!("{}{}", spacer, wrapped));
            }
        }
        lines.extend(vec!["".to_string(); self.padding_y as usize]);
        lines
    }

    fn invalidate(&mut self) {}
}
