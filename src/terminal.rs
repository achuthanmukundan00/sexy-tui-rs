/// Terminal abstraction layer. Port of src/terminal.ts (531 lines).

use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

use crossterm::cursor;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{self, ClearType};

use crate::terminal_colors::is_osc11_background_color_response;
use crate::terminal_image::get_capabilities;
use crate::keys::set_kitty_protocol_active;

const KITTY_KEYBOARD_PROTOCOL_QUERY: &str = "\x1b[>7u\x1b[?u\x1b[c";

/// Trait for terminal I/O implementations.
pub trait Terminal {
    /// Start the terminal with input and resize handlers.
    fn start(
        &mut self,
        on_input: Box<dyn FnMut(&str)>,
        on_resize: Box<dyn FnMut()>,
    );

    /// Stop the terminal and restore state.
    fn stop(&mut self);

    /// Write data to the terminal.
    fn write(&mut self, data: &str);

    /// Get current terminal width in columns.
    fn columns(&self) -> u16;

    /// Get current terminal height in rows.
    fn rows(&self) -> u16;

    /// Move cursor by N lines (negative = up).
    fn move_by(&mut self, lines: i16);

    /// Hide the terminal cursor.
    fn hide_cursor(&mut self);

    /// Show the terminal cursor.
    fn show_cursor(&mut self);

    /// Clear the current line.
    fn clear_line(&mut self);

    /// Clear from cursor position to end of screen.
    fn clear_from_cursor(&mut self);

    /// Clear the entire screen.
    fn clear_screen(&mut self);
}

/// Production terminal implementation using crossterm.
pub struct ProcessTerminal {
    stdout: io::Stdout,
    columns: u16,
    rows: u16,
    raw_mode: bool,
}

impl ProcessTerminal {
    pub fn new() -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(ProcessTerminal {
            stdout: io::stdout(),
            columns: cols,
            rows,
            raw_mode: false,
        })
    }

    /// Negotiate Kitty keyboard protocol support.
    fn negotiate_kitty_keyboard(&self) -> bool {
        let caps = get_capabilities();
        if caps.kitty_keyboard {
            // Send Kitty keyboard protocol query
            let mut stdout = io::stdout();
            let _ = execute!(stdout, crossterm::style::Print(KITTY_KEYBOARD_PROTOCOL_QUERY));
            let _ = stdout.flush();
            return true;
        }
        false
    }
}

impl Terminal for ProcessTerminal {
    fn start(
        &mut self,
        mut on_input: Box<dyn FnMut(&str)>,
        mut on_resize: Box<dyn FnMut()>,
    ) {
        // Enable raw mode
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        self.raw_mode = true;

        // Hide cursor
        execute!(self.stdout, cursor::Hide).ok();

        // Enable bracketed paste
        execute!(self.stdout, crossterm::style::Print("\x1b[?2004h")).ok();

        // Negotiate Kitty keyboard protocol
        let kitty = self.negotiate_kitty_keyboard();
        set_kitty_protocol_active(kitty);

        // Spawn input reader thread
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(key_event) => {
                            // Only process press events, not repeats
                            if key_event.kind == KeyEventKind::Repeat {
                                continue;
                            }
                            let data = key_to_string(&key_event);
                            if tx.send(data).is_err() {
                                break;
                            }
                        }
                        Event::Resize(cols, rows) => {
                            if tx.send(format!("RESIZE:{}:{}", cols, rows)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        // Process input events
        for data in rx {
            if data.starts_with("RESIZE:") {
                let parts: Vec<&str> = data.split(':').collect();
                if parts.len() == 3 {
                    if let (Ok(cols), Ok(rows)) = (parts[1].parse(), parts[2].parse()) {
                        self.columns = cols;
                        self.rows = rows;
                        on_resize();
                    }
                }
            } else {
                // Filter OSC 11 responses (handled internally)
                if is_osc11_background_color_response(&data) {
                    continue;
                }
                on_input(&data);
            }
        }
    }

    fn stop(&mut self) {
        execute!(self.stdout, cursor::Show).ok();
        execute!(self.stdout, crossterm::style::Print("\x1b[?2004l")).ok();
        terminal::disable_raw_mode().ok();
        self.raw_mode = false;
    }

    fn write(&mut self, data: &str) {
        let _ = self.stdout.write_all(data.as_bytes());
        let _ = self.stdout.flush();
    }

    fn columns(&self) -> u16 { self.columns }
    fn rows(&self) -> u16 { self.rows }

    fn move_by(&mut self, lines: i16) {
        if lines < 0 {
            execute!(self.stdout, cursor::MoveUp((-lines) as u16)).ok();
        } else {
            execute!(self.stdout, cursor::MoveDown(lines as u16)).ok();
        }
    }

    fn hide_cursor(&mut self) {
        execute!(self.stdout, cursor::Hide).ok();
    }

    fn show_cursor(&mut self) {
        execute!(self.stdout, cursor::Show).ok();
    }

    fn clear_line(&mut self) {
        execute!(self.stdout, crossterm::terminal::Clear(ClearType::CurrentLine)).ok();
    }

    fn clear_from_cursor(&mut self) {
        execute!(self.stdout, crossterm::terminal::Clear(ClearType::FromCursorDown)).ok();
    }

    fn clear_screen(&mut self) {
        execute!(self.stdout, crossterm::terminal::Clear(ClearType::All)).ok();
    }
}

/// Convert a crossterm KeyEvent to a string representation.
fn key_to_string(event: &event::KeyEvent) -> String {
    use crossterm::event::{KeyCode, KeyModifiers};

    let mut result = String::new();

    // Kitty protocol format: ESC [ codepoint ; modifier u
    if event.modifiers.contains(KeyModifiers::CONTROL)
        || event.modifiers.contains(KeyModifiers::ALT)
        || event.modifiers.contains(KeyModifiers::SUPER)
        || event.modifiers.contains(KeyModifiers::SHIFT)
    {
        let mut mod_val = 1u8; // 1-indexed
        if event.modifiers.contains(KeyModifiers::SHIFT) { mod_val += 1; }
        if event.modifiers.contains(KeyModifiers::ALT) { mod_val += 2; }
        if event.modifiers.contains(KeyModifiers::CONTROL) { mod_val += 4; }
        if event.modifiers.contains(KeyModifiers::SUPER) { mod_val += 8; }

        match event.code {
            KeyCode::Char(c) => {
                return format!("\x1b[{};{}u", c as u32, mod_val);
            }
            KeyCode::Enter => return format!("\x1b[13;{}u", mod_val),
            KeyCode::Tab => return format!("\x1b[9;{}u", mod_val),
            KeyCode::Backspace => return format!("\x1b[127;{}u", mod_val),
            KeyCode::Esc => return format!("\x1b[27;{}u", mod_val),
            _ => {}
        }
    }

    // Plain key events
    match event.code {
        KeyCode::Char(c) => result.push(c),
        KeyCode::Enter => result.push('\r'),
        KeyCode::Tab => result.push('\t'),
        KeyCode::Backspace => result.push('\x7f'),
        KeyCode::Esc => result.push('\x1b'),
        KeyCode::Up => result.push_str("\x1b[A"),
        KeyCode::Down => result.push_str("\x1b[B"),
        KeyCode::Left => result.push_str("\x1b[D"),
        KeyCode::Right => result.push_str("\x1b[C"),
        KeyCode::Home => result.push_str("\x1b[H"),
        KeyCode::End => result.push_str("\x1b[F"),
        KeyCode::Delete => result.push_str("\x1b[3~"),
        KeyCode::Insert => result.push_str("\x1b[2~"),
        KeyCode::PageUp => result.push_str("\x1b[5~"),
        KeyCode::PageDown => result.push_str("\x1b[6~"),
        KeyCode::F(n) => {
            if n <= 12 {
                result.push_str(&format!("\x1b[{}~", match n {
                    1 => "11", 2 => "12", 3 => "13", 4 => "14", 5 => "15",
                    6 => "17", 7 => "18", 8 => "19", 9 => "20", 10 => "21",
                    11 => "23", 12 => "24",
                    _ => unreachable!(),
                }));
            }
        }
        _ => {}
    }

    result
}
