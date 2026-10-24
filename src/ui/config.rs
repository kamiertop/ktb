//! Shared visual settings. Tool pages remain independent and consume these values.
use gpui_kit::FontWeight;

// Static bold face is embedded and registered before component initialization.
pub const FONT_FAMILY: &str = "Noto Sans CJK SC";
pub const CODE_FONT_FAMILY: &str = "Liberation Mono";

// Pixel sizes; keep body and editor text consistent across tools.
pub const BODY_SIZE: f32 = 16.0;
pub const EDITOR_SIZE: f32 = BODY_SIZE;
pub const LABEL_SIZE: f32 = 14.0;
pub const TITLE_SIZE: f32 = 28.0;
pub const SIDEBAR_IDENTITY_SIZE: f32 = 36.0;
pub const GITHUB_URL: &str = "https://github.com/";
pub const FONT_WEIGHT: FontWeight = FontWeight::BOLD;

pub const BACKGROUND: u32 = 0xf7f6f2;
pub const SIDEBAR: u32 = 0xefeee8;
pub const SURFACE: u32 = 0xffffff;
pub const INK: u32 = 0x111811;
pub const MUTED: u32 = 0x303b30;
pub const LINE: u32 = 0xcbd0c5;
pub const ACCENT: u32 = 0x36532b;
pub const ERROR: u32 = 0xa43e30;
pub const ERROR_BACKGROUND: u32 = 0xfdece8;
pub const SUCCESS_BACKGROUND: u32 = 0xebeee5;

pub const ON_ACCENT: u32 = 0xffffff;
pub const CONTENT_TOP: f32 = 8.0;
pub const CONTENT_HORIZONTAL: f32 = 20.0;
pub const CONTENT_GAP: f32 = 12.0;

// Sidebar navigation states, separate from action-button colors.
pub const NAV_WIDTH: f32 = 216.0;
pub const NAV_SELECTED: u32 = 0xdbeafe;
pub const NAV_SELECTED_TEXT: u32 = 0x1e40af;
pub const SEGMENTED_BACKGROUND: u32 = 0xdbeafe;

// Editor surface height, excluding compact labels and controls.
pub const TEXT_EDITOR_HEIGHT: f32 = 320.0;

// JSON shares the light application palette.
pub const JSON_EDITOR_SIZE: f32 = 15.0;
pub const JSON_EDITOR_BACKGROUND: u32 = SURFACE;
pub const JSON_EDITOR_FOREGROUND: u32 = INK;

// Controls switch immediately; avoid decorative sliding transitions.
pub const REDUCE_MOTION: bool = true;

// Cover previews share the available width; cap their height on large displays.
pub const COVER_PREVIEW_MAX_HEIGHT: f32 = 320.0;
pub const COVER_PREVIEW_MIN_HEIGHT: f32 = 160.0;
pub const COVER_PAGE_CONTROLS_HEIGHT: f32 = 400.0;

// Window frame and title-bar controls share the same radius.
pub const WINDOW_RADIUS: f32 = 16.0;
pub const TITLE_BAR_HEIGHT: f32 = 34.0;
