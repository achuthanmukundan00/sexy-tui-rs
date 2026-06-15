use unicode_width::UnicodeWidthChar;
use unicode_segmentation::UnicodeSegmentation;

// =============================================================================
// ANSI Code Extraction
// =============================================================================

/// Extract an ANSI escape sequence starting at `pos` in `str`.
/// Returns (code, length) or None if no sequence starts at pos.
pub fn extract_ansi_code(str: &str, pos: usize) -> Option<(&str, usize)> {
    let bytes = str.as_bytes();
    if pos >= bytes.len() || bytes[pos] != 0x1b {
        return None;
    }

    let next = bytes.get(pos + 1)?;

    match next {
        // CSI: ESC [ ... m/G/K/H/J
        b'[' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                let b = bytes[j];
                if matches!(b, b'm' | b'G' | b'K' | b'H' | b'J') {
                    return Some((&str[pos..=j], j + 1 - pos));
                }
                j += 1;
            }
            None
        }

        // OSC: ESC ] ... BEL or ESC ] ... ST
        b']' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    return Some((&str[pos..=j], j + 1 - pos));
                }
                if bytes[j] == 0x1b && bytes.get(j + 1) == Some(&b'\\') {
                    return Some((&str[pos..=j + 1], j + 2 - pos));
                }
                j += 1;
            }
            None
        }

        // APC: ESC _ ... BEL or ESC _ ... ST
        b'_' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    return Some((&str[pos..=j], j + 1 - pos));
                }
                if bytes[j] == 0x1b && bytes.get(j + 1) == Some(&b'\\') {
                    return Some((&str[pos..=j + 1], j + 2 - pos));
                }
                j += 1;
            }
            None
        }

        _ => None,
    }
}

// =============================================================================
// ANSI Code Tracker
// =============================================================================

/// Track active ANSI SGR codes to preserve styling across line breaks.
#[derive(Debug, Clone)]
pub struct AnsiCodeTracker {
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    inverse: bool,
    hidden: bool,
    strikethrough: bool,
    fg_color: Option<String>,
    bg_color: Option<String>,
    active_hyperlink: Option<String>, // OSC 8 hyperlink open sequence
}

impl AnsiCodeTracker {
    pub fn new() -> Self {
        AnsiCodeTracker {
            bold: false, dim: false, italic: false, underline: false,
            blink: false, inverse: false, hidden: false, strikethrough: false,
            fg_color: None, bg_color: None, active_hyperlink: None,
        }
    }

    /// Process an ANSI code and update tracker state.
    pub fn process(&mut self, code: &str) {
        // OSC 8 hyperlink
        if code.starts_with("\x1b]8;") {
            let body = &code[4..];
            let terminator_len = if code.ends_with("\x1b\\") { 2 } else if code.ends_with('\x07') { 1 } else { 0 };
            let body = &body[..body.len() - terminator_len];
            if let Some(sep_idx) = body.find(';') {
                let url = &body[sep_idx + 1..];
                if url.is_empty() {
                    self.active_hyperlink = None;
                } else {
                    self.active_hyperlink = Some(code.to_string());
                }
            }
            return;
        }

        if !code.ends_with('m') {
            return;
        }

        let params = &code[2..code.len() - 1]; // strip "\x1b[" and "m"
        if params.is_empty() || params == "0" {
            // Full reset
            *self = AnsiCodeTracker::new();
            return;
        }

        for param in params.split(';') {
            let mut parts = param.split(':');
            let num: u8 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            match num {
                0 => *self = AnsiCodeTracker::new(),
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                5 => self.blink = true,
                7 => self.inverse = true,
                8 => self.hidden = true,
                9 => self.strikethrough = true,
                21..=22 => { self.bold = false; self.dim = false; }
                23 => self.italic = false,
                24 => self.underline = false,
                25 => self.blink = false,
                27 => self.inverse = false,
                28 => self.hidden = false,
                29 => self.strikethrough = false,
                // Foreground colors
                30..=37 => self.fg_color = Some(num.to_string()),
                38 => self.fg_color = Some(format!("38;{}", parts.next().unwrap_or(""))),
                39 => self.fg_color = None,
                // Background colors
                40..=47 => self.bg_color = Some(num.to_string()),
                48 => self.bg_color = Some(format!("48;{}", parts.next().unwrap_or(""))),
                49 => self.bg_color = None,
                // Bright foreground
                90..=97 => self.fg_color = Some(num.to_string()),
                // Bright background
                100..=107 => self.bg_color = Some(num.to_string()),
                _ => {}
            }
        }
    }

    /// Get the currently active ANSI codes as a string.
    pub fn get_active_codes(&self) -> String {
        let mut codes = String::new();

        // Reopen hyperlink if active
        if let Some(ref hl) = self.active_hyperlink {
            codes.push_str(hl);
        }

        let mut sgr: Vec<String> = Vec::new();
        if self.bold { sgr.push("1".into()); }
        if self.dim { sgr.push("2".into()); }
        if self.italic { sgr.push("3".into()); }
        if self.underline { sgr.push("4".into()); }
        if self.blink { sgr.push("5".into()); }
        if self.inverse { sgr.push("7".into()); }
        if self.hidden { sgr.push("8".into()); }
        if self.strikethrough { sgr.push("9".into()); }
        if let Some(ref fg) = self.fg_color { sgr.push(fg.clone()); }
        if let Some(ref bg) = self.bg_color { sgr.push(bg.clone()); }

        if !sgr.is_empty() {
            codes.push_str(&format!("\x1b[{}m", sgr.join(";")));
        }

        codes
    }

    /// Get a reset sequence for line endings (resets underline but preserves background).
    pub fn get_line_end_reset(&self) -> Option<String> {
        if self.underline {
            Some("\x1b[24m".into()) // Reset underline only
        } else {
            None
        }
    }
}

impl Default for AnsiCodeTracker {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// Visible Width
// =============================================================================

/// Normalize Thai/Lao AM vowels for terminal output.
pub fn normalize_terminal_output(str: &str) -> String {
    str.replace('\u{0e33}', "\u{0e4d}\u{0e32}")
        .replace('\u{0eb3}', "\u{0ecd}\u{0eb2}")
}

/// Calculate the visible width of a string in terminal columns.
/// Strips ANSI escape codes before measuring.
pub fn visible_width(str: &str) -> usize {
    if str.is_empty() {
        return 0;
    }

    // Fast path: pure ASCII printable
    if str.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return str.len();
    }

    // Strip ANSI codes
    let clean = strip_ansi(str);

    // Calculate width using grapheme clusters
    let mut width = 0;
    for grapheme in clean.graphemes(true) {
        width += grapheme_width(grapheme);
    }

    width
}

fn strip_ansi(str: &str) -> String {
    if !str.contains('\x1b') {
        return str.to_string();
    }

    let mut result = String::with_capacity(str.len());
    let bytes = str.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b {
            if let Some((_, len)) = extract_ansi_code(str, i) {
                i += len;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

fn grapheme_width(grapheme: &str) -> usize {
    if grapheme == "\t" {
        return 3; // Tab width
    }

    // Get the first character
    let first_char = grapheme.chars().next().unwrap_or('\0');

    // Zero-width characters
    if is_zero_width(first_char) {
        return 0;
    }

    // Use unicode-width for standard characters
    let w = UnicodeWidthChar::width(first_char).unwrap_or(1);

    // Emoji and wide characters
    if could_be_emoji(grapheme) {
        return 2;
    }

    w
}

fn is_zero_width(c: char) -> bool {
    matches!(c,
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | // Zero-width
        '\u{0300}'..='\u{036F}' | // Combining diacritical marks
        '\u{0483}'..='\u{0489}' |
        '\u{0591}'..='\u{05BD}' |
        '\u{0610}'..='\u{061A}' |
        '\u{064B}'..='\u{065F}' |
        '\u{0670}' |
        '\u{06D6}'..='\u{06DC}' |
        '\u{06DF}'..='\u{06E4}' |
        '\u{06E7}'..='\u{06E8}' |
        '\u{06EA}'..='\u{06ED}' |
        '\u{0711}' |
        '\u{0730}'..='\u{074A}' |
        '\u{07A6}'..='\u{07B0}' |
        '\u{0900}'..='\u{0902}' |
        '\u{093A}'..='\u{093C}' |
        '\u{0941}'..='\u{0948}' |
        '\u{094D}' |
        '\u{0951}'..='\u{0957}' |
        '\u{0962}'..='\u{0963}' |
        '\u{0981}'..='\u{0983}' |
        '\u{09BC}' |
        '\u{09C1}'..='\u{09C4}' |
        '\u{09CD}' |
        '\u{09E2}'..='\u{09E3}' |
        '\u{0A01}'..='\u{0A03}' |
        '\u{0A3C}' |
        '\u{0A41}'..='\u{0A42}' | '\u{0A47}'..='\u{0A48}' | '\u{0A4B}'..='\u{0A4D}' |
        '\u{0A70}'..='\u{0A71}' |
        '\u{0A81}'..='\u{0A83}' |
        '\u{0ABC}' |
        '\u{0AC1}'..='\u{0AC5}' | '\u{0AC7}'..='\u{0AC8}' |
        '\u{0ACD}' |
        '\u{0AE2}'..='\u{0AE3}' |
        '\u{0B01}'..='\u{0B03}' |
        '\u{0B3C}' |
        '\u{0B3F}' |
        '\u{0B41}'..='\u{0B44}' |
        '\u{0B4D}' |
        '\u{0B56}' |
        '\u{0B82}' |
        '\u{0BC0}' |
        '\u{0BCD}' |
        '\u{0C3E}'..='\u{0C40}' |
        '\u{0C46}'..='\u{0C48}' | '\u{0C4A}'..='\u{0C4D}' |
        '\u{0C55}'..='\u{0C56}' |
        '\u{0CBC}' |
        '\u{0CBF}' |
        '\u{0CC6}' |
        '\u{0CCC}'..='\u{0CCD}' |
        '\u{0CE2}'..='\u{0CE3}' |
        '\u{0D41}'..='\u{0D44}' |
        '\u{0D4D}' |
        '\u{0DCA}' |
        '\u{0DD2}'..='\u{0DD4}' |
        '\u{0DD6}' |
        '\u{0E31}' |
        '\u{0E34}'..='\u{0E3A}' |
        '\u{0E47}'..='\u{0E4E}' |
        '\u{0EB1}' |
        '\u{0EB4}'..='\u{0EB9}' |
        '\u{0EBB}'..='\u{0EBC}' |
        '\u{0EC8}'..='\u{0ECD}' |
        '\u{0F18}'..='\u{0F19}' |
        '\u{0F35}' |
        '\u{0F37}' |
        '\u{0F39}' |
        '\u{0F71}'..='\u{0F7E}' |
        '\u{0F80}'..='\u{0F84}' |
        '\u{0F86}'..='\u{0F87}' |
        '\u{0F90}'..='\u{0F97}' |
        '\u{0F99}'..='\u{0FBC}' |
        '\u{0FC6}' |
        '\u{102D}'..='\u{1030}' |
        '\u{1032}'..='\u{1037}' |
        '\u{1039}'..='\u{103A}' |
        '\u{103D}'..='\u{103E}' |
        '\u{1058}'..='\u{1059}' |
        '\u{105E}'..='\u{1060}' |
        '\u{1071}'..='\u{1074}' |
        '\u{1082}' |
        '\u{1085}'..='\u{1086}' |
        '\u{108D}' |
        '\u{109D}' |
        '\u{1160}'..='\u{11FF}' | // Hangul Jungseong/Jongseong
        '\u{135D}'..='\u{135F}' |
        '\u{1712}'..='\u{1714}' |
        '\u{1732}'..='\u{1734}' |
        '\u{1752}'..='\u{1753}' |
        '\u{1772}'..='\u{1773}' |
        '\u{17B4}'..='\u{17B5}' |
        '\u{17B7}'..='\u{17BD}' |
        '\u{17C6}' |
        '\u{17C9}'..='\u{17D3}' |
        '\u{17DD}' |
        '\u{180B}'..='\u{180D}' |
        '\u{1885}'..='\u{1886}' |
        '\u{18A9}' |
        '\u{1920}'..='\u{1922}' |
        '\u{1927}'..='\u{1928}' |
        '\u{1932}' |
        '\u{1939}'..='\u{193B}' |
        '\u{1A17}'..='\u{1A18}' |
        '\u{1B00}'..='\u{1B03}' |
        '\u{1B34}' |
        '\u{1B36}'..='\u{1B3A}' |
        '\u{1B3C}' |
        '\u{1B42}' |
        '\u{1B6B}'..='\u{1B73}' |
        '\u{1DC0}'..='\u{1DFF}' |
        '\u{200E}'..='\u{200F}' |
        '\u{202A}'..='\u{202E}' |
        '\u{2060}'..='\u{2064}' |
        '\u{2066}'..='\u{206F}' |
        '\u{20D0}'..='\u{20F0}' |
        '\u{2CEF}'..='\u{2CF1}' |
        '\u{2D7F}' |
        '\u{2DE0}'..='\u{2DFF}' |
        '\u{A66F}'..='\u{A672}' |
        '\u{A674}'..='\u{A67D}' |
        '\u{A69E}'..='\u{A69F}' |
        '\u{A6F0}'..='\u{A6F1}' |
        '\u{A802}' |
        '\u{A806}' |
        '\u{A80B}' |
        '\u{A825}'..='\u{A826}' |
        '\u{A8C4}'..='\u{A8C5}' |
        '\u{A8E0}'..='\u{A8F1}' |
        '\u{A926}'..='\u{A92D}' |
        '\u{A947}'..='\u{A951}' |
        '\u{A980}'..='\u{A982}' |
        '\u{A9B3}' |
        '\u{A9B6}'..='\u{A9B9}' |
        '\u{A9BC}' |
        '\u{AA29}'..='\u{AA2E}' |
        '\u{AA31}'..='\u{AA32}' |
        '\u{AA35}'..='\u{AA36}' |
        '\u{AA43}' |
        '\u{AA4C}' |
        '\u{AAB0}' |
        '\u{AAB2}'..='\u{AAB4}' |
        '\u{AAB7}'..='\u{AAB8}' |
        '\u{AABE}'..='\u{AABF}' |
        '\u{AAC1}' |
        '\u{AAEC}'..='\u{AAED}' |
        '\u{AAF6}' |
        '\u{ABE5}' |
        '\u{ABE8}' |
        '\u{ABED}' |
        '\u{FB1E}' |
        '\u{FE00}'..='\u{FE0F}' |
        '\u{FE20}'..='\u{FE2F}' |
        '\u{101FD}' |
        '\u{10A01}'..='\u{10A03}' | '\u{10A05}'..='\u{10A06}' |
        '\u{10A0C}'..='\u{10A0F}' |
        '\u{10A38}'..='\u{10A3A}' |
        '\u{10A3F}' |
        '\u{11001}' |
        '\u{11038}'..='\u{11046}' |
        '\u{11080}'..='\u{11081}' |
        '\u{110B3}'..='\u{110B6}' |
        '\u{110B9}'..='\u{110BA}' |
        '\u{11100}'..='\u{11102}' |
        '\u{11127}'..='\u{1112B}' |
        '\u{1112D}'..='\u{11134}' |
        '\u{11180}'..='\u{11181}' |
        '\u{111B6}'..='\u{111BE}' |
        '\u{116AB}' |
        '\u{116AD}' |
        '\u{116B0}'..='\u{116B5}' |
        '\u{116B7}' |
        '\u{16F8F}'..='\u{16F92}' |
        '\u{1D167}'..='\u{1D169}' |
        '\u{1D173}'..='\u{1D17A}' |
        '\u{1D185}'..='\u{1D18B}' |
        '\u{1D1AA}'..='\u{1D1AD}' |
        '\u{1D242}'..='\u{1D244}' |
        '\u{E0001}' |
        '\u{E0020}'..='\u{E007F}' |
        '\u{E0100}'..='\u{E01EF}'
    )
}

fn could_be_emoji(grapheme: &str) -> bool {
    let cp = grapheme.chars().next().map(|c| c as u32).unwrap_or(0);
    // Broad heuristic matching the TS version
    (0x1f000..=0x1fbff).contains(&cp)  // Emoji and Pictographs
        || (0x2300..=0x23ff).contains(&cp) // Misc Technical
        || (0x2600..=0x27bf).contains(&cp) // Misc Symbols, Dingbats
        || (0x2b50..=0x2b55).contains(&cp) // Stars/Circles
        || grapheme.contains('\u{FE0F}') // Variation Selector-16
        || grapheme.graphemes(true).count() > 2 // Multi-codepoint sequences
}

// =============================================================================
// Truncate to Width
// =============================================================================

/// Truncate a string to fit within `max_width` visible columns.
/// Preserves ANSI escape codes. Adds ellipsis if truncated.
pub fn truncate_to_width(str: &str, max_width: usize, ellipsis: Option<&str>) -> String {
    let ellipsis = ellipsis.unwrap_or("…");
    let ellipsis_width = visible_width(ellipsis);
    let available = max_width.saturating_sub(ellipsis_width);

    if visible_width(str) <= max_width {
        return str.to_string();
    }

    // Walk graphemes until we exceed available width
    let clean = strip_ansi(str);
    let mut visible = 0;
    let _byte_pos = 0;

    // We need to walk the original string (with ANSI) but count widths from clean
    let mut clean_idx = 0;
    let chars: Vec<char> = str.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        // Check for ANSI sequence
        let remaining: String = chars[i..].iter().collect();
        if let Some((code, _len)) = extract_ansi_code(&remaining, 0) {
            // Convert len from byte length to char count
            let char_len = code.chars().count();
            result.push_str(code);
            i += char_len;
            continue;
        }

        if clean_idx >= clean.len() {
            break;
        }

        // Get the next grapheme from clean
        let graphemes: Vec<&str> = clean.graphemes(true).collect();
        if let Some(g) = graphemes.get(clean_idx / clean.chars().count()) {
            let w = grapheme_width(g);
            clean_idx += g.len();

            if visible + w > available {
                break;
            }
            visible += w;
            // Copy this grapheme's chars from original
            let g_chars: Vec<char> = g.chars().collect();
            for _ in 0..g_chars.len() {
                if i < chars.len() {
                    result.push(chars[i]);
                    i += 1;
                }
            }
        } else {
            break;
        }
    }

    result.push_str("\x1b[0m"); // SGR reset
    result.push_str(ellipsis);
    result.push_str("\x1b[0m"); // reset after ellipsis

    result
}

// =============================================================================
// Word Wrap with ANSI
// =============================================================================

/// Word-wrap text to fit within `width` visible columns, preserving ANSI codes.
pub fn wrap_text_with_ansi(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let mut result: Vec<String> = Vec::new();
    let mut tracker = AnsiCodeTracker::new();

    for input_line in &lines {
        let prefix = if !result.is_empty() {
            tracker.get_active_codes()
        } else {
            String::new()
        };
        let wrapped = wrap_single_line(&format!("{}{}", prefix, input_line), width);
        for line in wrapped {
            result.push(line);
        }
        update_tracker_from_text(input_line, &mut tracker);
    }

    if result.is_empty() {
        vec![String::new()]
    } else {
        result
    }
}

fn wrap_single_line(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }

    if visible_width(line) <= width {
        return vec![line.to_string()];
    }

    let mut wrapped: Vec<String> = Vec::new();
    let mut tracker = AnsiCodeTracker::new();
    let tokens = split_into_tokens_with_ansi(line);
    let mut current_line = String::new();
    let mut current_visible = 0;

    for token in &tokens {
        let token_visible = visible_width(token);
        let is_whitespace = token.trim().is_empty();

        // Token itself is too long — break it
        if token_visible > width && !is_whitespace {
            if !current_line.is_empty() {
                if let Some(ref reset) = tracker.get_line_end_reset() {
                    current_line.push_str(reset);
                }
                wrapped.push(current_line.clone());
                current_line.clear();
                current_visible = 0;
            }
            let broken = break_long_word(token, width, &tracker);
            let len = broken.len();
            for (idx, line) in broken.into_iter().enumerate() {
                if idx < len - 1 {
                    wrapped.push(line);
                } else {
                    current_line = line;
                    current_visible = visible_width(&current_line);
                }
            }
            continue;
        }

        let total_needed = current_visible + token_visible;

        if total_needed > width && current_visible > 0 {
            let trimmed = current_line.trim_end().to_string();
            if let Some(ref reset) = tracker.get_line_end_reset() {
                wrapped.push(format!("{}{}", trimmed, reset));
            } else {
                wrapped.push(trimmed);
            }
            if is_whitespace {
                current_line = tracker.get_active_codes();
                current_visible = 0;
            } else {
                current_line = format!("{}{}", tracker.get_active_codes(), token);
                current_visible = token_visible;
            }
        } else {
            current_line.push_str(token);
            current_visible += token_visible;
        }

        update_tracker_from_text(token, &mut tracker);
    }

    if !current_line.is_empty() {
        wrapped.push(current_line.trim_end().to_string());
    }

    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn split_into_tokens_with_ansi(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        let remaining: String = chars[i..].iter().collect();
        if let Some((code, _len)) = extract_ansi_code(&remaining, 0) {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(code.to_string());
            i += code.chars().count();
            continue;
        }

        if chars[i].is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            let mut ws = String::new();
            while i < chars.len() && chars[i].is_whitespace() {
                let remaining: String = chars[i..].iter().collect();
                if extract_ansi_code(&remaining, 0).is_some() {
                    break;
                }
                ws.push(chars[i]);
                i += 1;
            }
            tokens.push(ws);
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn break_long_word(word: &str, width: usize, tracker: &AnsiCodeTracker) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = tracker.get_active_codes();
    let mut current_width = 0;

    // Walk graphemes, handling ANSI codes
    let clean = strip_ansi(word);
    let graphemes: Vec<&str> = clean.graphemes(true).collect();
    let _g_idx = 0;
    let _byte_pos = 0;

    for g in &graphemes {
        let w = grapheme_width(g);

        if current_width + w > width && current_width > 0 {
            if let Some(ref reset) = tracker.get_line_end_reset() {
                current_line.push_str(reset);
            }
            lines.push(current_line);
            current_line = tracker.get_active_codes();
            current_width = 0;
        }

        // Find this grapheme in the original (with ANSI) and copy it
        // Simplified: just push the grapheme chars
        current_line.push_str(g);
        current_width += w;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![word.to_string()]
    } else {
        lines
    }
}

fn update_tracker_from_text(text: &str, tracker: &mut AnsiCodeTracker) {
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        let remaining: String = chars[i..].iter().collect();
        if let Some((code, _len)) = extract_ansi_code(&remaining, 0) {
            tracker.process(code);
            i += code.chars().count();
            continue;
        }
        i += 1;
    }
}

// =============================================================================
// Character Classification
// =============================================================================

pub const PUNCTUATION_CHARS: &str = "(){}[]<>.,;:'\"!?+-=*/\\|&%^$#@~`";

/// Check if the first character of a string is punctuation.
pub fn is_punctuation_char(s: &str) -> bool {
    s.chars().next().is_some_and(|c| PUNCTUATION_CHARS.contains(c))
}

/// Check if a string is only whitespace.
pub fn is_whitespace_str(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_width_ascii() {
        assert_eq!(visible_width("hello"), 5);
        assert_eq!(visible_width(""), 0);
    }

    #[test]
    fn test_visible_width_ansi() {
        assert_eq!(visible_width("\x1b[31mhello\x1b[0m"), 5);
    }

    #[test]
    fn test_visible_width_emoji() {
        assert_eq!(visible_width("🎉"), 2);
    }

    #[test]
    fn test_truncate_to_width() {
        let result = truncate_to_width("hello world", 8, None);
        assert!(result.contains("…"));
        assert!(result.starts_with("hello"));
    }

    #[test]
    fn test_truncate_no_truncation() {
        assert_eq!(truncate_to_width("hi", 10, None), "hi");
    }

    #[test]
    fn test_wrap_text_simple() {
        let result = wrap_text_with_ansi("hello world", 5);
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn test_extract_ansi_csi() {
        let s = "\x1b[31mhello";
        let (code, len) = extract_ansi_code(s, 0).unwrap();
        assert_eq!(code, "\x1b[31m");
        assert_eq!(len, 5);
    }
}
