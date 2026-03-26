use iced::Color;

pub struct Theme;

impl Theme {
    pub const BG:           Color = Color { r: 0.04, g: 0.04, b: 0.06, a: 1.0 };
    pub const PANE_BG:      Color = Color { r: 0.05, g: 0.05, b: 0.08, a: 1.0 };
    pub const PANE_BORDER:  Color = Color { r: 0.12, g: 0.12, b: 0.18, a: 1.0 };
    pub const RING_WAITING: Color = Color { r: 0.39, g: 0.60, b: 0.98, a: 1.0 };
    pub const SIDEBAR_BG:   Color = Color { r: 0.07, g: 0.07, b: 0.10, a: 1.0 };
    pub const TEXT_PRIMARY: Color = Color { r: 0.89, g: 0.91, b: 0.94, a: 1.0 };
    pub const TEXT_DIM:     Color = Color { r: 0.39, g: 0.44, b: 0.53, a: 1.0 };
    pub const BADGE_ACTIVE: Color = Color { r: 0.20, g: 0.83, b: 0.60, a: 1.0 };
    pub const ACCENT:       Color = Color { r: 0.33, g: 0.73, b: 1.0,  a: 1.0 };
}
