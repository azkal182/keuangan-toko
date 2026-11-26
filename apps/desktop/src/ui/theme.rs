//! Definisi tema dan palet visual Native Desktop (Windows Look & Feel).
//! Didesain dengan estetika desktop profesional: grid lines tajam, baris kompak,
//! font tabular, header kontras, dan panel status klasik.

use gpui::{Hsla, rgb, rgba};

pub struct Theme;

impl Theme {
    // Backgrounds
    pub const BG_WINDOW: u32 = 0xF0F2F5; // Window background abu-abu desktop
    pub const BG_SURFACE: u32 = 0xFFFFFF; // Permukaan input & tabel
    pub const BG_HEADER: u32 = 0xEAEEF3; // Header DBGrid & Toolbar
    pub const BG_ROW_ALT: u32 = 0xF8FAFC; // Baris tabel selang-seling (zebra)
    pub const BG_ROW_SELECTED: u32 = 0x0078D7; // Windows blue selection
    pub const BG_ROW_HOVER: u32 = 0xEDF2F7; // Hover baris tabel
    pub const BG_STATUS_BAR: u32 = 0xE2E8F0; // Status bar abu-abu klasik
    pub const BG_MENU_BAR: u32 = 0xF1F5F9; // Menu bar

    // Borders & Lines
    pub const BORDER_LIGHT: u32 = 0xE2E8F0;
    pub const BORDER_DARK: u32 = 0x94A3B8; // Border luar window/dialog
    pub const GRID_LINE: u32 = 0xCBD5E1; // Grid line tajam DBGrid

    // Text Colors
    pub const TEXT_PRIMARY: u32 = 0x0F172A; // Hitam teks utama
    pub const TEXT_SECONDARY: u32 = 0x475569; // Abu-abu keterangan
    pub const TEXT_MUTED: u32 = 0x64748B; // Muted label
    pub const TEXT_SELECTED: u32 = 0xFFFFFF; // Teks saat baris dipilih
    pub const TEXT_LINK: u32 = 0x0284C7;

    // Financial / Status Badges
    pub const COLOR_DEBIT: u32 = 0xB91C1C; // Merah debet / tagihan
    pub const COLOR_CREDIT: u32 = 0x15803D; // Hijau lunas / kredit
    pub const COLOR_DEPOSIT: u32 = 0x1D4ED8; // Biru deposit / saldo kredit
    pub const COLOR_WARNING: u32 = 0xD97706; // Kuning amber perhatian

    // Dialog & Titlebar
    pub const TITLEBAR_BG: u32 = 0x1E293B; // Dark slate titlebar modal
    pub const TITLEBAR_TEXT: u32 = 0xFFFFFF;

    // Button colors
    pub const BTN_DEFAULT_BG: u32 = 0xF8FAFC;
    pub const BTN_DEFAULT_BORDER: u32 = 0xCBD5E1;
    pub const BTN_PRIMARY_BG: u32 = 0x0078D7;
    pub const BTN_PRIMARY_TEXT: u32 = 0xFFFFFF;
    pub const BTN_DANGER_BG: u32 = 0xDC2626;
    pub const BTN_DANGER_TEXT: u32 = 0xFFFFFF;

    pub fn hsla(hex: u32) -> Hsla {
        rgb(hex).into()
    }

    pub fn hsla_alpha(hex: u32, alpha: f32) -> Hsla {
        rgba(hex | ((alpha * 255.0) as u32)).into()
    }
}

pub fn format_idr(amount: i64) -> String {
    let s = amount.abs().to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push('.');
        }
        result.push(c);
    }
    if amount < 0 {
        format!("-{}", result)
    } else {
        result
    }
}
