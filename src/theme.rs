//! Y2K Clinical Theme for egui
//! 
//! Light/Dark theme with technical aesthetic

use egui::Color32;

#[derive(Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color32,
    pub window: Color32,
    pub header: Color32,
    pub panel: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub border: Color32,
    pub accent_on: Color32,
    pub accent_off: Color32,
}

impl Theme {
    pub const LIGHT: Self = Self {
        bg: Color32::from_rgb(0xe8, 0xe8, 0xe8),
        window: Color32::from_rgb(0xf7, 0xf7, 0xf7),
        header: Color32::from_rgb(0xff, 0xff, 0xff),
        panel: Color32::from_rgb(0xff, 0xff, 0xff),
        text: Color32::from_rgb(0x2a, 0x2a, 0x2a),
        text_dim: Color32::from_rgb(0x88, 0x88, 0x88),
        border: Color32::from_rgb(0xa0, 0xa0, 0xa0),
        accent_on: Color32::from_rgb(0x2a, 0x2a, 0x2a),
        accent_off: Color32::from_rgb(0xd0, 0xd0, 0xd0),
    };

    pub const DARK: Self = Self {
        bg: Color32::from_rgb(0x0f, 0x0f, 0x0f),
        window: Color32::from_rgb(0x1a, 0x1a, 0x1a),
        header: Color32::from_rgb(0x14, 0x14, 0x14),
        panel: Color32::from_rgb(0x22, 0x22, 0x22),
        text: Color32::from_rgb(0xe0, 0xe0, 0xe0),
        text_dim: Color32::from_rgb(0x5c, 0x5c, 0x5c),
        border: Color32::from_rgb(0x33, 0x33, 0x33),
        accent_on: Color32::from_rgb(0x00, 0xbc, 0xd4), // Cyan
        accent_off: Color32::from_rgb(0x33, 0x33, 0x33),
    };

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::LIGHT,
            ThemeMode::Dark => Self::DARK,
        }
    }
}

/// Apply theme to egui visuals
pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let mut visuals = egui::Visuals::dark();
    
    visuals.panel_fill = theme.window;
    visuals.window_fill = theme.panel;
    visuals.extreme_bg_color = theme.bg;
    
    visuals.widgets.noninteractive.fg_stroke.color = theme.text;
    visuals.widgets.inactive.fg_stroke.color = theme.text_dim;
    visuals.widgets.active.fg_stroke.color = theme.text;
    visuals.widgets.hovered.fg_stroke.color = theme.text;
    
    visuals.widgets.noninteractive.bg_fill = theme.panel;
    visuals.widgets.inactive.bg_fill = theme.panel;
    
    visuals.selection.bg_fill = theme.accent_on;
    
    ctx.set_visuals(visuals);
}
