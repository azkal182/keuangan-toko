//! Native Desktop Design System & Theme (Windows Enterprise Desktop Style)
//! Dirancang menyerupai aplikasi desktop akuntansi native (WinUI / WPF / Accurate Desktop / SSMS):
//! - System Menu Bar tipis (26px) dengan permukaan netral
//! - Left Navigation Explorer Pane (Tree/Module Navigator) dengan border pemisah tegas
//! - Contextual Command Toolbar (36px) yang hanya menampilkan tombol sesuai halaman aktif
//! - Native DBGrid dengan kolom indikator baris, garis sel (gridlines) presisi, dan format akuntansi `(Rp ...)`

use gpui::{Hsla, rgb, rgba};

pub struct Theme;

impl Theme {
    // Native Window & Panel Surfaces
    pub const BG_SYSTEM_MENU: u32 = 0xE2E8F0; // Baris menu sistem paling atas (Slate 200)
    pub const BG_SIDEBAR: u32 = 0xF1F5F9; // Panel navigasi kiri (Slate 100)
    pub const BG_SIDEBAR_ACTIVE: u32 = 0xDBEAFE; // Item navigasi aktif (Blue 100)
    pub const BG_TOOLBAR: u32 = 0xF8FAFC; // Command bar kontekstual per halaman
    pub const BG_APP: u32 = 0xE5E7EB; // Latar workspace desktop
    pub const BG_CARD: u32 = 0xFFFFFF; // Permukaan DBGrid & panel data
    pub const BG_HEADER: u32 = 0xF1F5F9; // Header kolom DBGrid native
    pub const BG_ROW_ALT: u32 = 0xF8FAFC; // Zebra striping DBGrid
    pub const BG_ROW_HOVER: u32 = 0xEFF6FF; // Hover baris DBGrid
    pub const BG_ROW_SELECTED: u32 = 0xDBEAFE; // Seleksi baris DBGrid aktif
    pub const BG_STATUS_BAR: u32 = 0x1E293B; // Status bar bawah native tegas (Slate 800)

    // Borders & Gridlines (Tegas selayaknya aplikasi desktop native)
    pub const BORDER_LIGHT: u32 = 0xE2E8F0;
    pub const BORDER_DEFAULT: u32 = 0xCBD5E1; // Border panel & tombol toolbar native
    pub const BORDER_STRONG: u32 = 0x94A3B8; // Border input & frame aktif
    pub const GRID_LINE: u32 = 0xE2E8F0; // Garis sel vertikal/horizontal DBGrid

    // Primary & Accent Colors
    pub const PRIMARY: u32 = 0x1D4ED8; // Native Desktop Blue (Blue 700)
    pub const PRIMARY_HOVER: u32 = 0x1E40AF; // Blue 800
    pub const PRIMARY_LIGHT: u32 = 0xEFF6FF; // Blue 50
    pub const PRIMARY_BORDER: u32 = 0x93C5FD; // Blue 300

    // Text Hierarchy
    pub const TEXT_PRIMARY: u32 = 0x0F172A; // Slate 900
    pub const TEXT_SECONDARY: u32 = 0x334155; // Slate 700
    pub const TEXT_MUTED: u32 = 0x475569; // Slate 600
    pub const TEXT_LIGHT: u32 = 0x64748B; // Slate 500
    pub const TEXT_ON_DARK: u32 = 0xF8FAFC;

    // Financial & Status Colors
    pub const BADGE_DEBIT_TEXT: u32 = 0xB91C1C; // Red 700
    pub const BADGE_DEBIT_BG: u32 = 0xFEF2F2;
    pub const BADGE_DEBIT_BORDER: u32 = 0xFCA5A5;

    pub const BADGE_CREDIT_TEXT: u32 = 0x15803D; // Green 700
    pub const BADGE_CREDIT_BG: u32 = 0xF0FDF4;
    pub const BADGE_CREDIT_BORDER: u32 = 0x86EFAC;

    pub const BADGE_LUNAS_TEXT: u32 = 0x0E7490; // Cyan 700 (seperti web Lunas badge)
    pub const BADGE_LUNAS_BG: u32 = 0xECFEFF;
    pub const BADGE_LUNAS_BORDER: u32 = 0xA5F3FC;

    pub const BADGE_DEPOSIT_TEXT: u32 = 0x1D4ED8; // Blue 700
    pub const BADGE_DEPOSIT_BG: u32 = 0xEFF6FF;
    pub const BADGE_DEPOSIT_BORDER: u32 = 0x93C5FD;

    pub const BADGE_WARNING_TEXT: u32 = 0xB45309; // Amber 700
    pub const BADGE_WARNING_BG: u32 = 0xFFFBEB;
    pub const BADGE_WARNING_BORDER: u32 = 0xFCD34D;

    pub const BADGE_CRITICAL_TEXT: u32 = 0x991B1B; // Red 800
    pub const BADGE_CRITICAL_BG: u32 = 0xFEE2E2;
    pub const BADGE_CRITICAL_BORDER: u32 = 0xFCA5A5;

    pub const BADGE_EXCEPTION_TEXT: u32 = 0x6B21A8; // Purple 800 (Unknown / Future Date)
    pub const BADGE_EXCEPTION_BG: u32 = 0xFAF5FF;
    pub const BADGE_EXCEPTION_BORDER: u32 = 0xD8B4FE;

    // Modal / Native Dialog Window Styling
    pub const MODAL_HEADER_BG: u32 = 0x1E293B; // Native window titlebar slate
    pub const MODAL_BORDER: u32 = 0x64748B;

    // Focus & Input Ring
    pub const FOCUS_BORDER: u32 = 0x2563EB; // Blue 600 - High contrast active border
    pub const FOCUS_RING: u32 = 0xDBEAFE; // Blue 100 - Glow halo effect for focused field
    pub const FOCUS_BG: u32 = 0xFFFFFF; // White crisp background
    pub const CURSOR_COLOR: u32 = 0x1E40AF; // Blue 800 - Native blinking cursor bar
    pub const MODAL_BACKDROP: u32 = 0x0F172A;

    pub fn hsla(hex: u32) -> Hsla {
        rgb(hex).into()
    }

    pub fn hsla_alpha(hex: u32, alpha: f32) -> Hsla {
        rgba(hex | ((alpha * 255.0) as u32)).into()
    }
}

/// Format angka integer IDR menjadi string ribuan bertitik (contoh: `1.938.200`).
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

/// Format nominal rupiah standar (contoh: `Rp 1.938.200`).
pub fn format_rupiah(amount: i64) -> String {
    if amount < 0 {
        format!("-Rp {}", format_idr(amount.abs()))
    } else {
        format!("Rp {}", format_idr(amount))
    }
}

/// Format saldo berjalan akuntansi sesuai FR-004:
/// Nilai negatif diformat dalam tanda kurung akuntansi `(Rp 18.900)`.
pub fn format_accounting_rupiah(amount: i64) -> String {
    if amount < 0 {
        format!("(Rp {})", format_idr(amount.abs()))
    } else {
        format!("Rp {}", format_idr(amount))
    }
}
