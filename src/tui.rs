/// Core TUI implementation with differential rendering.
/// Port of @earendil-works/pi-tui src/tui.ts (1641 lines).

use std::cell::RefCell;
use std::rc::Rc;

use crate::terminal::Terminal;
use crate::terminal_image::{delete_all_kitty_images, is_image_line};
use crate::utils::visible_width;

/// Zero-width APC escape sequence used as a cursor position marker.
pub const CURSOR_MARKER: &str = "\x1b_\\";

// =============================================================================
// Component Trait
// =============================================================================

/// Component interface — all UI elements must implement this.
pub trait Component {
    /// Render the component to lines for the given viewport width.
    fn render(&self, width: u16) -> Vec<String>;

    /// Handle keyboard input when component has focus.
    fn handle_input(&mut self, _data: &str) {}

    /// If true, component receives key release events (Kitty protocol).
    fn wants_key_release(&self) -> bool {
        false
    }

    /// Invalidate any cached rendering state.
    fn invalidate(&mut self);
}

/// Components that can receive focus and display a hardware cursor for IME.
pub trait Focusable {
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
}

/// Check if a component implements Focusable.
pub fn is_focusable(_component: &dyn Component) -> bool {
    // In stable Rust, we can't directly check for trait implementation on trait objects.
    // We use a workaround: try to cast to Any and check.
    false // Placeholder — requires type registry or downcast
}

// =============================================================================
// Container
// =============================================================================

/// Container that groups child components vertically.
pub struct Container {
    children: Vec<Box<dyn Component>>,
    focused_child: Option<usize>,
    cached_width: Option<u16>,
    cached_lines: Option<Vec<String>>,
}

impl Container {
    pub fn new() -> Self {
        Container {
            children: Vec::new(),
            focused_child: None,
            cached_width: None,
            cached_lines: None,
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }

    pub fn remove_child(&mut self, child_idx: usize) {
        if child_idx < self.children.len() {
            self.children.remove(child_idx);
            if self.focused_child == Some(child_idx) {
                self.focused_child = None;
            }
        }
    }

    pub fn set_focus(&mut self, idx: Option<usize>) {
        self.focused_child = idx;
    }

    pub fn focused_child_mut(&mut self) -> Option<&mut Box<dyn Component>> {
        self.focused_child.and_then(|i| self.children.get_mut(i))
    }
}

impl Component for Container {
    fn render(&self, width: u16) -> Vec<String> {
        let mut lines = Vec::new();
        for child in &self.children {
            lines.extend(child.render(width));
        }
        lines
    }

    fn handle_input(&mut self, data: &str) {
        if let Some(idx) = self.focused_child {
            if let Some(child) = self.children.get_mut(idx) {
                child.handle_input(data);
            }
        }
    }

    fn wants_key_release(&self) -> bool {
        if let Some(idx) = self.focused_child {
            self.children.get(idx).map_or(false, |c| c.wants_key_release())
        } else {
            false
        }
    }

    fn invalidate(&mut self) {
        self.cached_width = None;
        self.cached_lines = None;
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Overlay Support
// =============================================================================

/// Anchor position for overlays.
#[derive(Debug, Clone, Copy)]
pub enum OverlayAnchor {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    LeftCenter,
    RightCenter,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Margin values for overlays.
#[derive(Debug, Clone, Copy)]
pub struct OverlayMargin {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl OverlayMargin {
    pub fn all(value: u16) -> Self {
        OverlayMargin { top: value, right: value, bottom: value, left: value }
    }
}

/// Options for focusing/unfocusing an overlay.
pub struct OverlayUnfocusOptions {
    pub target: Option<Box<dyn Component>>,
}

/// Options for creating an overlay.
pub struct OverlayOptions {
    pub width: Option<u16>,
    pub min_width: Option<u16>,
    pub max_height: Option<u16>,
    pub anchor: OverlayAnchor,
    pub offset_x: i16,
    pub offset_y: i16,
    pub row: Option<u16>,
    pub col: Option<u16>,
    pub margin: Option<OverlayMargin>,
    pub non_capturing: bool,
}

impl Default for OverlayOptions {
    fn default() -> Self {
        OverlayOptions {
            width: None,
            min_width: None,
            max_height: None,
            anchor: OverlayAnchor::Center,
            offset_x: 0,
            offset_y: 0,
            row: None,
            col: None,
            margin: None,
            non_capturing: false,
        }
    }
}

/// Handle to an active overlay.
#[derive(Clone)]
pub struct OverlayHandle {
    pub id: usize,
    hidden: bool,
    focused: bool,
}

impl OverlayHandle {
    pub fn hide(&mut self) {
        self.hidden = true;
    }

    pub fn show(&mut self) {
        self.hidden = false;
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self, _options: Option<OverlayUnfocusOptions>) {
        self.focused = false;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }
}

// =============================================================================
// TUI — Main Interface
// =============================================================================

/// Main TUI instance managing the render loop.
pub struct TUI<'a> {
    terminal: Box<dyn Terminal + 'a>,
    root: Container,
    overlays: Vec<(Rc<RefCell<OverlayHandle>>, Box<dyn Component>)>,
    next_overlay_id: usize,
    previous_frame: Vec<String>,
    first_render: bool,
    running: bool,
    input_listeners: Vec<Box<dyn FnMut(&str) -> Option<String> + 'a>>,
}

impl<'a> TUI<'a> {
    pub fn new(terminal: Box<dyn Terminal + 'a>) -> Self {
        TUI {
            terminal,
            root: Container::new(),
            overlays: Vec::new(),
            next_overlay_id: 0,
            previous_frame: Vec::new(),
            first_render: true,
            running: false,
            input_listeners: Vec::new(),
        }
    }

    /// Add a component to the root container.
    pub fn add_child(&mut self, child: Box<dyn Component>) {
        self.root.add_child(child);
    }

    /// Remove a component from the root container.
    pub fn remove_child(&mut self, idx: usize) {
        self.root.remove_child(idx);
    }

    /// Set focus to a specific child.
    pub fn set_focus(&mut self, idx: Option<usize>) {
        self.root.set_focus(idx);
    }

    /// Show an overlay on top of the current content.
    pub fn show_overlay(&mut self, component: Box<dyn Component>, options: OverlayOptions) -> Rc<RefCell<OverlayHandle>> {
        let id = self.next_overlay_id;
        self.next_overlay_id += 1;
        let handle = Rc::new(RefCell::new(OverlayHandle { id, hidden: false, focused: !options.non_capturing }));
        self.overlays.push((handle.clone(), component));
        handle
    }

    /// Hide the topmost overlay.
    pub fn hide_overlay(&mut self) {
        self.overlays.pop();
    }

    /// Check if any visible overlay is active.
    pub fn has_overlay(&self) -> bool {
        self.overlays.iter().any(|(h, _)| !h.borrow().hidden)
    }

    /// Add an input listener for global key handling.
    pub fn add_input_listener(&mut self, f: Box<dyn FnMut(&str) -> Option<String> + 'a>) {
        self.input_listeners.push(f);
    }

    /// Request a re-render at the next opportunity.
    pub fn request_render(&mut self) {
        // Trigger immediate re-render
        if self.running {
            self.render_frame();
        }
    }

    /// Start the TUI render loop.
    pub fn start(&mut self) {
        self.running = true;
        let (_cols, _) = (self.terminal.columns(), self.terminal.rows());

        // Perform first render
        self.render_frame();

        // Input/event loop is handled externally by the caller
        // (matching pi-tui's architecture where the consumer drives the loop)
    }

    /// Stop the TUI render loop.
    pub fn stop(&mut self) {
        self.running = false;
        self.terminal.clear_screen();
    }

    /// Process input data. Should be called by the consumer's event loop.
    pub fn handle_input(&mut self, data: &str) {
        // Run input listeners first
        for listener in &mut self.input_listeners {
            if let Some(_modified) = listener(data) {
                // Listener consumed/modified the input
                return;
            }
        }

        // Route to focused overlay or root
        let has_capturing_overlay = self.overlays.iter().any(|(h, _)| h.borrow().focused && !h.borrow().hidden);

        if has_capturing_overlay {
            if let Some((_, component)) = self.overlays.last_mut() {
                component.handle_input(data);
            }
        } else {
            self.root.handle_input(data);
        }

        self.request_render();
    }

    /// Render the current frame using the differential rendering algorithm.
    fn render_frame(&mut self) {
        let width = self.terminal.columns();
        let _height = self.terminal.rows();

        let mut new_lines: Vec<String> = Vec::new();

        // Render root container
        for line in self.root.render(width) {
            new_lines.push(ensure_line_width(&line, width));
        }

        // Render any visible overlays on top
        for (handle, component) in &self.overlays {
            if handle.borrow().hidden {
                continue;
            }
            let overlay_lines = component.render(width);
            // Overlay is rendered on top — simple overlay (would need proper compositing)
            // For now, replace root lines with overlay lines at the same position
            new_lines = overlay_lines;
        }

        // Apply SGR reset and OSC 8 reset per line
        new_lines = new_lines.into_iter()
            .map(|line| format!("{}\x1b[0m\x1b]8;;\x1b\\", line))
            .collect();

        // Differential rendering
        if self.first_render {
            // Strategy 1: First render — output all lines
            self.write_all_lines(&new_lines);
            self.first_render = false;
        } else if self.previous_frame.is_empty() {
            // Fallback to full render
            self.write_all_lines(&new_lines);
        } else {
            // Strategy 3: Incremental update
            let first_changed = self.previous_frame.iter()
                .zip(&new_lines)
                .position(|(prev, new)| prev != new)
                .unwrap_or(self.previous_frame.len().min(new_lines.len()));

            if first_changed == 0 && self.previous_frame.len() != new_lines.len() {
                // Lines changed count — full re-render
                self.terminal.clear_screen();
                self.write_all_lines(&new_lines);
            } else if first_changed < self.previous_frame.len() {
                // Move cursor to first changed line, clear to end, write tail
                self.terminal.write("\x1b[?2026h"); // Begin sync output
                // Move cursor to correct row (simplified — assumes cursor at bottom)
                let diff = self.previous_frame.len() - first_changed;
                if diff > 0 {
                    let move_up = format!("\x1b[{}A", diff);
                    self.terminal.write(&move_up);
                }
                self.terminal.clear_from_cursor();
                for line in &new_lines[first_changed..] {
                    self.terminal.write(line);
                    self.terminal.write("\n");
                }
                // Trim leftover lines if new frame is shorter
                if new_lines.len() < self.previous_frame.len() {
                    self.terminal.clear_from_cursor();
                }
                self.terminal.write("\x1b[?2026l"); // End sync output
            } // else no change, skip
        }

        self.previous_frame = new_lines;
    }

    fn write_all_lines(&mut self, lines: &[String]) {
        self.terminal.write("\x1b[?2026h"); // Begin sync output
        // Delete any existing Kitty images on full render
        if !self.first_render {
            // Check if any previous lines had images
            let has_images = self.previous_frame.iter().any(|l| is_image_line(l));
            if has_images {
                self.terminal.write(&delete_all_kitty_images());
            }
        }
        for line in lines {
            self.terminal.write(line);
            self.terminal.write("\n");
        }
        self.terminal.write("\x1b[?2026l"); // End sync output
    }
}

/// Ensure a line is exactly `width` columns wide by padding with spaces.
fn ensure_line_width(line: &str, width: u16) -> String {
    let visible = visible_width(line) as u16;
    if visible < width {
        format!("{}{}", line, " ".repeat((width - visible) as usize))
    } else if visible > width {
        // Truncation should have happened in render(). Error if not.
        line[..line.len().min(width as usize)].to_string()
    } else {
        line.to_string()
    }
}
