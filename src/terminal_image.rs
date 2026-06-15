/// Terminal image support (Kitty/iTerm2 graphics protocols).
/// Port of src/terminal-image.ts (483 lines).

use std::sync::atomic::{AtomicU32, Ordering};

/// Supported image protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    Kitty,
    ITerm2,
    None,
}

/// Detected terminal capabilities.
#[derive(Debug, Clone)]
pub struct TerminalCapabilities {
    pub kitty_graphics: bool,
    pub iterm2_images: bool,
    pub sync_output: bool,
    pub kitty_keyboard: bool,
    pub true_color: bool,
    pub nerd_font: bool,
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        TerminalCapabilities {
            kitty_graphics: false,
            iterm2_images: false,
            sync_output: false,
            kitty_keyboard: false,
            true_color: false,
            nerd_font: false,
        }
    }
}

/// Terminal cell dimensions.
#[derive(Debug, Clone, Copy)]
pub struct CellDimensions {
    pub width_px: u32,
    pub height_px: u32,
}

impl Default for CellDimensions {
    fn default() -> Self {
        CellDimensions { width_px: 10, height_px: 20 }
    }
}

/// Image dimensions.
#[derive(Debug, Clone, Copy)]
pub struct ImageDimensions {
    pub width_px: u32,
    pub height_px: u32,
}

/// Options for rendering an image.
#[derive(Debug, Clone)]
pub struct ImageRenderOptions {
    pub max_width_cells: Option<u32>,
    pub max_height_cells: Option<u32>,
    pub filename: Option<String>,
}

static NEXT_IMAGE_ID: AtomicU32 = AtomicU32::new(1);

/// Allocate a unique image ID for Kitty graphics protocol.
pub fn allocate_image_id() -> u32 {
    NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Check if a line contains a Kitty image sequence.
pub fn is_image_line(line: &str) -> bool {
    line.contains("\x1b_G")
}

/// Encode image data in Kitty graphics protocol format.
pub fn encode_kitty(
    image_id: u32,
    base64_data: &str,
    dims: ImageDimensions,
    opts: &ImageRenderOptions,
    cell_dims: CellDimensions,
) -> String {
    let rows = calculate_image_rows(dims, opts, cell_dims);
    let cols = if let Some(max_w) = opts.max_width_cells {
        ((dims.width_px as f64 / cell_dims.width_px as f64).ceil() as u32)
            .min(max_w)
    } else {
        (dims.width_px as f64 / cell_dims.width_px as f64).ceil() as u32
    };

    format!(
        "\x1b_Ga=T,f=100,i={},s={},v={},c={},r={};{}\x1b\\",
        image_id, dims.width_px, dims.height_px, cols, rows, base64_data
    )
}

/// Encode image data in iTerm2 inline image format.
pub fn encode_iterm2(
    base64_data: &str,
    dims: ImageDimensions,
    opts: &ImageRenderOptions,
    _cell_dims: CellDimensions,
) -> String {
    let width = opts.max_width_cells.unwrap_or(dims.width_px);
    let height = calculate_image_rows(dims, opts, CellDimensions::default()) as u32;
    format!(
        "\x1b]1337;File=inline=1;width={}px;height={}px;preserveAspectRatio=1:{}\x07",
        width, height, base64_data
    )
}

/// Calculate the number of terminal rows an image will occupy.
pub fn calculate_image_rows(
    dims: ImageDimensions,
    opts: &ImageRenderOptions,
    cell_dims: CellDimensions,
) -> u32 {
    let max_h = opts.max_height_cells.unwrap_or(u32::MAX);
    let aspect = dims.width_px as f64 / dims.height_px as f64;
    let cell_aspect = cell_dims.width_px as f64 / cell_dims.height_px as f64;
    let adjusted_height = (dims.width_px as f64 / cell_dims.width_px as f64 / aspect * cell_aspect).ceil() as u32;
    adjusted_height.min(max_h).max(1)
}

/// Delete a specific Kitty image from the terminal.
pub fn delete_kitty_image(image_id: u32) -> String {
    format!("\x1b_Ga=d,d=I,i={}\x1b\\", image_id)
}

/// Delete all Kitty images from the terminal.
pub fn delete_all_kitty_images() -> String {
    "\x1b_Ga=d\x1b\\".to_string()
}

/// Create an OSC 8 hyperlink.
pub fn hyperlink(text: &str, url: &str) -> String {
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
}

/// Fallback text for image display on unsupported terminals.
pub fn image_fallback(mime_type: &str, dims: Option<ImageDimensions>, filename: Option<&str>) -> String {
    let name = filename.unwrap_or("image");
    if let Some(d) = dims {
        format!("[{}: {} {}×{}px]", name, mime_type, d.width_px, d.height_px)
    } else {
        format!("[{}: {}]", name, mime_type)
    }
}

use std::sync::Mutex;
static CAPABILITIES: Mutex<Option<TerminalCapabilities>> = Mutex::new(None);
static CELL_DIMS: Mutex<Option<CellDimensions>> = Mutex::new(None);

/// Detect terminal capabilities.
pub fn detect_capabilities() -> TerminalCapabilities {
    let mut caps = TerminalCapabilities::default();

    // Check for Kitty graphics support
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("kitty") || term.contains("ghostty") || term.contains("wezterm") {
            caps.kitty_graphics = true;
            caps.sync_output = true;
            caps.kitty_keyboard = true;
        }
    }
    if let Ok(program) = std::env::var("TERM_PROGRAM") {
        if program == "iTerm.app" {
            caps.iterm2_images = true;
        }
    }
    if let Ok(color) = std::env::var("COLORTERM") {
        if color == "truecolor" || color == "24bit" {
            caps.true_color = true;
        }
    }

    caps
}

/// Get cached capabilities.
pub fn get_capabilities() -> TerminalCapabilities {
    let mut guard = CAPABILITIES.lock().unwrap();
    if guard.is_none() {
        *guard = Some(detect_capabilities());
    }
    guard.clone().unwrap()
}

/// Reset the capabilities cache.
pub fn reset_capabilities_cache() {
    *CAPABILITIES.lock().unwrap() = None;
}

/// Set capabilities explicitly.
pub fn set_capabilities(caps: TerminalCapabilities) {
    *CAPABILITIES.lock().unwrap() = Some(caps);
}

/// Get cell dimensions (for image size calculations).
pub fn get_cell_dimensions() -> CellDimensions {
    CELL_DIMS.lock().unwrap().unwrap_or_default()
}

/// Set cell dimensions.
pub fn set_cell_dimensions(dims: CellDimensions) {
    *CELL_DIMS.lock().unwrap() = Some(dims);
}

/// Get PNG image dimensions from base64 data.
pub fn get_png_dimensions(_base64_data: &str) -> Option<ImageDimensions> {
    // PNG header parsing: read IHDR chunk
    // For now, return a default
    None
}

/// Get JPEG image dimensions from base64 data.
pub fn get_jpeg_dimensions(_base64_data: &str) -> Option<ImageDimensions> {
    None
}

/// Get GIF image dimensions from base64 data.
pub fn get_gif_dimensions(_base64_data: &str) -> Option<ImageDimensions> {
    None
}

/// Get WebP image dimensions from base64 data.
pub fn get_webp_dimensions(_base64_data: &str) -> Option<ImageDimensions> {
    None
}

/// Get image dimensions from base64 data (auto-detect format).
pub fn get_image_dimensions(base64_data: &str, mime_type: &str) -> Option<ImageDimensions> {
    match mime_type {
        "image/png" => get_png_dimensions(base64_data),
        "image/jpeg" | "image/jpg" => get_jpeg_dimensions(base64_data),
        "image/gif" => get_gif_dimensions(base64_data),
        "image/webp" => get_webp_dimensions(base64_data),
        _ => None,
    }
}

/// Render an image to terminal escape sequences.
pub fn render_image(
    base64_data: &str,
    mime_type: &str,
    opts: &ImageRenderOptions,
) -> String {
    let caps = get_capabilities();
    let cell_dims = get_cell_dimensions();
    let dims = get_image_dimensions(base64_data, mime_type);

    if let Some(d) = dims {
        if caps.kitty_graphics {
            let id = allocate_image_id();
            return encode_kitty(id, base64_data, d, opts, cell_dims);
        }
        if caps.iterm2_images {
            return encode_iterm2(base64_data, d, opts, cell_dims);
        }
    }

    image_fallback(mime_type, dims, opts.filename.as_deref())
}
