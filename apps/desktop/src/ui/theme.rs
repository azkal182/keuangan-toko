//! Modern Enterprise Desktop Design System & Theme
//! Menghadirkan antarmuka kelas enterprise yang modern, profesional, dan elegan:
//! - Palet Slate / Indigo / Corporate Blue kontemporer
//! - Tipografi bersih dengan visual hierarchy yang jelas
//! - Badges berbingkai halus dengan sudut membulat modern
//! - Grid lines yang presisi tanpa kesan kuno/kaku
//! - Status bar minimalis dengan indikator status sistem bercahaya

use gpui::{Hsla, rgb, rgba};

pub struct Theme;

impl Theme {
    // Canvas & Surfaces
    pub const BG_APP: u32 = 0xF8FAFC; // Latar belakang utama (Slate 50)
    pub const BG_CARD: u32 = 0xFFFFFF; // Permukaan kartu & tabel (Putih murni)
    pub const BG_NAVBAR: u32 = 0x0F172A; // Top App Bar Enterprise (Dark Slate 900)
    pub const BG_NAVBAR_ACTIVE: u32 = 0x1E293B; // Item navbar aktif (Slate 800)
    pub const BG_TOOLBAR: u32 = 0xFFFFFF; // Toolbar sub-header putih bersih
    pub const BG_HEADER: u32 = 0xF8FAFC; // Header DBGrid halus (Slate 50)
    pub const BG_ROW_ALT: u32 = 0xFAFCFD; // Striping baris tabel sangat halus
    pub const BG_ROW_HOVER: u32 = 0xF1F5F9; // Hover baris tabel (Slate 100)
    pub const BG_ROW_SELECTED: u32 = 0xEFF6FF; // Soft Enterprise Blue selection (Blue 50)
    pub const BG_STATUS_BAR: u32 = 0xFFFFFF; // Status bar bawah modern

    // Borders & Dividers
    pub const BORDER_LIGHT: u32 = 0xF1F5F9; // Garis pembatas sangat halus (Slate 100)
    pub const BORDER_DEFAULT: u32 = 0xE2E8F0; // Border standar kartu/tabel (Slate 200)
    pub const BORDER_STRONG: u32 = 0xCBD5E1; // Border input field (Slate 300)
    pub const GRID_LINE: u32 = 0xF1F5F9; // Grid line tabel yang tidak menyakitkan mata

    // Primary & Accent Colors
    pub const PRIMARY: u32 = 0x2563EB; // Corporate Blue (Blue 600)
    pub const PRIMARY_HOVER: u32 = 0x1D4ED8; // Blue 700
    pub const PRIMARY_LIGHT: u32 = 0xEFF6FF; // Blue 50
    pub const PRIMARY_BORDER: u32 = 0xBFDBFE; // Blue 200

    // Text Hierarchy
    pub const TEXT_PRIMARY: u32 = 0x0F172A; // Slate 900 (teks utama tebal & kontras)
    pub const TEXT_SECONDARY: u32 = 0x334155; // Slate 700 (teks konten)
    pub const TEXT_MUTED: u32 = 0x64748B; // Slate 500 (label dan keterangan)
    pub const TEXT_LIGHT: u32 = 0x94A3B8; // Slate 400 (placeholder / ikon sekunder)
    pub const TEXT_ON_DARK: u32 = 0xF8FAFC; // Slate 50 (teks di atas navbar gelap)
    pub const TEXT_LINK: u32 = 0x2563EB; // Blue 600

    // Financial & Status Badges (Pill Styling)
    pub const BADGE_DEBIT_TEXT: u32 = 0xDC2626; // Red 600
    pub const BADGE_DEBIT_BG: u32 = 0xFEF2F2; // Red 50
    pub const BADGE_DEBIT_BORDER: u32 = 0xFECACA; // Red 200

    pub const BADGE_CREDIT_TEXT: u32 = 0x16A34A; // Green 600
    pub const BADGE_CREDIT_BG: u32 = 0xF0FDF4; // Green 50
    pub const BADGE_CREDIT_BORDER: u32 = 0xBBF7D0; // Green 200

    pub const BADGE_DEPOSIT_TEXT: u32 = 0x2563EB; // Blue 600
    pub const BADGE_DEPOSIT_BG: u32 = 0xEFF6FF; // Blue 50
    pub const BADGE_DEPOSIT_BORDER: u32 = 0xBFDBFE; // Blue 200

    pub const BADGE_WARNING_TEXT: u32 = 0xD97706; // Amber 600
    pub const BADGE_WARNING_BG: u32 = 0xFFFBEB; // Amber 50
    pub const BADGE_WARNING_BORDER: u32 = 0xFDE68A; // Amber 200

    pub const BADGE_CRITICAL_TEXT: u32 = 0x991B1B; // Red 800
    pub const BADGE_CRITICAL_BG: u32 = 0xFEE2E2; // Red 100
    pub const BADGE_CRITICAL_BORDER: u32 = 0xFCA5A5; // Red 300

    // Modal / Windowed Dialog Styling
    pub const MODAL_HEADER_BG: u32 = 0xFFFFFF; // Header modal modern putih bersih
    pub const MODAL_BORDER: u32 = 0xE2E8F0;
    pub const MODAL_BACKDROP: u32 = 0x0F172A; // Backdrop modern dark slate

    // Buttons
    pub const BTN_PRIMARY_BG: u32 = 0x2563EB;
    pub const BTN_PRIMARY_TEXT: u32 = 0xFFFFFF;
    pub const BTN_SECONDARY_BG: u32 = 0xF8FAFC;
    pub const BTN_SECONDARY_BORDER: u32 = 0xE2E8F0;
    pub const BTN_SECONDARY_TEXT: u32 = 0x334155;

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
