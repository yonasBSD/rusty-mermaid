//! Deterministic font selection for non-SVG backends.
//!
//! O(1) per character. No charmap probing, no allocation.
//! SVG delegates to the browser — this module is for raster, gpui, and vello.

// ── Canonical font family names ──
// All backends read from here. Single source of truth.

/// Primary monospace font for ASCII/Latin.
pub const PRIMARY_FONT: &str = "Intel One Mono";
/// Proportional fallback for Greek, Cyrillic.
pub const EXTENDED_TEXT_FONT: &str = "Noto Sans";
/// Monospace fallback for arrows, box drawing, math.
pub const MONOSPACE_FONT: &str = "Noto Sans Mono";
/// Symbol font for dingbats (☕ ✔ ✘ ★ ☆).
pub const DINGBATS_FONT: &str = "Noto Sans Symbols 2";
/// Arabic/Hebrew script font (Naskh style, closest to macOS Geeza Pro).
pub const ARABIC_FONT: &str = "Noto Naskh Arabic";

/// CSS font-family stack for SVG rendering.
pub const SVG_FONT_FAMILY: &str = "'Intel One Mono', 'SF Mono', 'Cascadia Code', 'JetBrains Mono', 'Fira Code', 'Consolas', 'Menlo', monospace";

/// Get the canonical font family name for a FontSlot.
pub const fn font_family_for_slot(slot: FontSlot) -> &'static str {
    match slot {
        FontSlot::Primary => PRIMARY_FONT,
        FontSlot::ExtendedText => EXTENDED_TEXT_FONT,
        FontSlot::Monospace => MONOSPACE_FONT,
        FontSlot::Dingbats => DINGBATS_FONT,
        FontSlot::Arabic => ARABIC_FONT,
        FontSlot::Cjk => "Noto Sans SC",
        FontSlot::Emoji => "Noto Color Emoji",
    }
}

/// Embedded font file paths (relative to raster/fonts/).
/// Backends that embed fonts use these.
pub const fn embedded_font_file(slot: FontSlot) -> Option<&'static str> {
    match slot {
        FontSlot::Primary => Some("IntelOneMono-Regular.ttf"),
        FontSlot::ExtendedText => Some("NotoSans-Regular.ttf"),
        FontSlot::Monospace => Some("NotoSansMono-Regular.ttf"),
        FontSlot::Dingbats => Some("NotoSansSymbols2-Regular.ttf"),
        FontSlot::Arabic => Some("NotoNaskhArabic-Regular.ttf"),
        FontSlot::Cjk => None,   // CDN only
        FontSlot::Emoji => None, // CDN only
    }
}

/// Which font handles a given character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSlot {
    /// Intel One Mono — ASCII, Latin, Latin Extended
    Primary,
    /// Noto Sans (proportional) — Greek, Cyrillic
    ExtendedText,
    /// Noto Sans Mono — arrows, box drawing, technical symbols
    Monospace,
    /// Noto Sans Symbols 2 — dingbats (☕ ✔ ✘ ★ ☆)
    Dingbats,
    /// Noto Naskh Arabic — Arabic, Persian, Urdu, Hebrew (Naskh style)
    Arabic,
    /// Noto Sans SC — CJK (Chinese, Japanese, Korean) — CDN only
    Cjk,
    /// Noto Color Emoji — emoji — CDN only
    Emoji,
}

impl FontSlot {
    #[inline]
    pub const fn is_embedded(self) -> bool {
        matches!(
            self,
            Self::Primary | Self::ExtendedText | Self::Monospace | Self::Dingbats | Self::Arabic
        )
    }

    #[inline]
    pub const fn is_external(self) -> bool {
        matches!(self, Self::Cjk | Self::Emoji)
    }
}

/// Determine which font slot handles a character. O(1), no allocation.
#[inline]
pub const fn font_for_char(ch: char) -> FontSlot {
    let cp = ch as u32;
    match cp {
        // ASCII + Latin-1 Supplement + Latin Extended-A/B
        0x0000..=0x024F => FontSlot::Primary,
        // Latin Extended Additional
        0x1E00..=0x1EFF => FontSlot::Primary,

        // Greek and Coptic + Greek Extended
        0x0370..=0x03FF | 0x1F00..=0x1FFF => FontSlot::ExtendedText,
        // Cyrillic + Cyrillic Supplement + Cyrillic Extended
        0x0400..=0x04FF | 0x0500..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => FontSlot::ExtendedText,

        // Arabic + Arabic Supplement + Arabic Extended + Arabic Presentation Forms
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF |
        0xFB50..=0xFDFF | 0xFE70..=0xFEFF => FontSlot::Arabic,
        // Hebrew
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => FontSlot::Arabic,

        // CJK Unified Ideographs + Extensions
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF |
        // CJK Compatibility Ideographs
        0xF900..=0xFAFF |
        // Hiragana + Katakana
        0x3040..=0x30FF | 0x31F0..=0x31FF |
        // Hangul
        0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F |
        // CJK Symbols and Punctuation
        0x3000..=0x303F |
        // Bopomofo
        0x3100..=0x312F |
        // Fullwidth Forms
        0xFF01..=0xFF60 => FontSlot::Cjk,

        // Emoji ranges
        0x1F300..=0x1F9FF | // Misc Symbols & Pictographs, Emoticons, etc.
        0x1FA00..=0x1FA6F | 0x1FA70..=0x1FAFF | // Extended-A, Extended-B
        0xFE00..=0xFE0F |   // Variation Selectors
        0x200D => FontSlot::Emoji, // ZWJ

        // General Punctuation, Superscripts, Currency — most monospace fonts have these
        0x2000..=0x20CF => FontSlot::Primary,
        // Letterlike Symbols, Number Forms
        0x2100..=0x214F => FontSlot::Primary,
        // Arrows — Noto Sans Mono has these
        0x2190..=0x21FF => FontSlot::Monospace,
        // Mathematical Operators
        0x2200..=0x22FF => FontSlot::Monospace,
        // Misc Technical, Control Pictures, OCR
        0x2300..=0x23FF => FontSlot::Monospace,
        // Enclosed Alphanumerics
        0x2460..=0x24FF => FontSlot::Monospace,
        // Box Drawing, Block Elements, Geometric Shapes
        0x2500..=0x25FF => FontSlot::Monospace,
        // Misc Symbols + Dingbats (☕ ✔ ✘ ★ ☆ etc.)
        0x2600..=0x27BF => FontSlot::Dingbats,
        // Supplemental Arrows, Misc Math Symbols
        0x27C0..=0x27EF | 0x2980..=0x29FF | 0x2B00..=0x2BFF => FontSlot::Dingbats,

        // Devanagari, Bengali, Tamil, etc.
        0x0900..=0x0DFF => FontSlot::ExtendedText,
        // Thai, Lao
        0x0E00..=0x0EFF => FontSlot::ExtendedText,

        // Everything else → ExtendedText (best effort)
        _ => FontSlot::ExtendedText,
    }
}

/// Check which external font slots are needed for a text.
/// Returns (needs_cjk, needs_emoji). O(n) single pass, no allocation.
#[inline]
pub fn detect_external_fonts(text: &str) -> (bool, bool) {
    let mut cjk = false;
    let mut emoji = false;
    for ch in text.chars() {
        match font_for_char(ch) {
            FontSlot::Cjk => cjk = true,
            FontSlot::Emoji => emoji = true,
            _ => {}
        }
        if cjk && emoji {
            break;
        } // early exit
    }
    (cjk, emoji)
}

/// CDN URLs for external fonts (WASM only).
pub const NOTO_SANS_SC_URL: &str =
    "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf";
pub const NOTO_COLOR_EMOJI_URL: &str =
    "https://cdn.jsdelivr.net/gh/googlefonts/noto-emoji@v2.047/fonts/NotoColorEmoji.ttf";

/// System font fallback paths for native platforms.
pub fn system_font_dirs() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/",
            "/System/Library/Fonts/Supplemental/",
            "/Library/Fonts/",
        ]
    } else if cfg!(target_os = "linux") {
        &["/usr/share/fonts/", "/usr/local/share/fonts/"]
    } else if cfg!(target_os = "windows") {
        &["C:\\Windows\\Fonts\\"]
    } else {
        &[]
    }
}

/// Find a system font file by family name.
pub fn find_system_font(family: &str) -> Option<std::path::PathBuf> {
    for dir in system_font_dirs() {
        let dir_path = std::path::Path::new(dir);
        if !dir_path.exists() {
            continue;
        }
        for ext in &["ttf", "ttc", "otf"] {
            let path = dir_path.join(format!("{family}.{ext}"));
            if path.exists() {
                return Some(path);
            }
            let path = dir_path.join(format!("{}.{ext}", family.replace(' ', "")));
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_primary() {
        assert_eq!(font_for_char('A'), FontSlot::Primary);
        assert_eq!(font_for_char('z'), FontSlot::Primary);
        assert_eq!(font_for_char('0'), FontSlot::Primary);
        assert_eq!(font_for_char(' '), FontSlot::Primary);
    }

    #[test]
    fn latin_extended_is_primary() {
        assert_eq!(font_for_char('é'), FontSlot::Primary);
        assert_eq!(font_for_char('ñ'), FontSlot::Primary);
        assert_eq!(font_for_char('ü'), FontSlot::Primary);
    }

    #[test]
    fn greek_is_extended_text() {
        assert_eq!(font_for_char('α'), FontSlot::ExtendedText);
        assert_eq!(font_for_char('β'), FontSlot::ExtendedText);
        assert_eq!(font_for_char('γ'), FontSlot::ExtendedText);
    }

    #[test]
    fn cyrillic_is_extended_text() {
        assert_eq!(font_for_char('П'), FontSlot::ExtendedText);
        assert_eq!(font_for_char('р'), FontSlot::ExtendedText);
    }

    #[test]
    fn arabic_is_arabic() {
        assert_eq!(font_for_char('م'), FontSlot::Arabic);
        assert_eq!(font_for_char('ر'), FontSlot::Arabic);
    }

    #[test]
    fn cjk_is_cjk() {
        assert_eq!(font_for_char('你'), FontSlot::Cjk);
        assert_eq!(font_for_char('好'), FontSlot::Cjk);
        assert_eq!(font_for_char('世'), FontSlot::Cjk);
    }

    #[test]
    fn symbols_are_dingbats() {
        assert_eq!(font_for_char('★'), FontSlot::Dingbats);
        assert_eq!(font_for_char('☆'), FontSlot::Dingbats);
        assert_eq!(font_for_char('→'), FontSlot::Monospace);
        assert_eq!(font_for_char('✔'), FontSlot::Dingbats);
        assert_eq!(font_for_char('✘'), FontSlot::Dingbats);
    }

    #[test]
    fn coffee_is_dingbats() {
        assert_eq!(font_for_char('☕'), FontSlot::Dingbats);
    }

    #[test]
    fn detect_external_ascii_only() {
        assert_eq!(detect_external_fonts("Hello World"), (false, false));
    }

    #[test]
    fn detect_external_cjk() {
        assert_eq!(detect_external_fonts("Hello 你好"), (true, false));
    }

    #[test]
    fn detect_external_emoji() {
        let (_, emoji) = detect_external_fonts("Hello 😀");
        assert!(emoji);
    }

    #[test]
    fn font_for_char_is_const() {
        // Compile-time verification that font_for_char is const
        const _: FontSlot = font_for_char('A');
        const _B: FontSlot = font_for_char('你');
    }
}
