use ratatui::style::Color;

pub struct Theme {
    pub bg: Color, pub surface: Color, pub surface_alt: Color,
    pub primary: Color, pub secondary: Color, pub accent: Color,
    pub success: Color, pub warning: Color, pub error: Color,
    pub text: Color, pub text_dim: Color, pub text_muted: Color,
    pub border: Color, pub border_focus: Color,
}

pub const DARK: Theme = Theme {
    bg: Color::Rgb(10, 12, 22),
    surface: Color::Rgb(16, 20, 34),
    surface_alt: Color::Rgb(24, 30, 50),
    primary: Color::Rgb(86, 156, 255),
    secondary: Color::Rgb(160, 120, 255),
    accent: Color::Rgb(72, 230, 200),
    success: Color::Rgb(50, 215, 145),
    warning: Color::Rgb(255, 185, 40),
    error: Color::Rgb(255, 85, 85),
    text: Color::Rgb(230, 235, 248),
    text_dim: Color::Rgb(110, 125, 155),
    text_muted: Color::Rgb(50, 60, 80),
    border: Color::Rgb(45, 55, 80),
    border_focus: Color::Rgb(86, 156, 255),
};
