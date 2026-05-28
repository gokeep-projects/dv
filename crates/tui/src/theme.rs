use ratatui::style::Color;

pub struct Theme {
    pub bg: Color, pub surface: Color, pub surface_alt: Color,
    pub primary: Color, pub secondary: Color, pub accent: Color,
    pub success: Color, pub warning: Color, pub error: Color,
    pub text: Color, pub text_dim: Color, pub text_muted: Color,
    pub border: Color, pub border_focus: Color,
}

pub const DARK: Theme = Theme {
    bg: Color::Rgb(8, 10, 18),
    surface: Color::Rgb(15, 18, 28),
    surface_alt: Color::Rgb(22, 26, 40),
    primary: Color::Rgb(80, 160, 255),
    secondary: Color::Rgb(150, 120, 255),
    accent: Color::Rgb(80, 230, 210),
    success: Color::Rgb(40, 220, 140),
    warning: Color::Rgb(250, 180, 30),
    error: Color::Rgb(255, 90, 90),
    text: Color::Rgb(235, 240, 250),
    text_dim: Color::Rgb(120, 135, 160),
    text_muted: Color::Rgb(55, 65, 85),
    border: Color::Rgb(40, 50, 72),
    border_focus: Color::Rgb(80, 160, 255),
};
