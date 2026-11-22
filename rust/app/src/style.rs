use iced::font::Weight;
use iced::widget::{button, container, slider, text_input};
use iced::{Background, Border, Color, Font, Theme, border};

pub const INK: Color = Color::from_rgb8(0x1a, 0x0b, 0x12);
pub const FG: Color = Color::from_rgb8(0xf5, 0xf5, 0xf6);
pub const MUTED: Color = Color::from_rgba8(255, 255, 255, 0.55);
pub const FAINT: Color = Color::from_rgba8(255, 255, 255, 0.38);
pub const PINK: Color = Color::from_rgb8(0xff, 0x4f, 0x8b);
pub const PINK_SOFT: Color = Color::from_rgba8(0xff, 0x4f, 0x8b, 0.85);
pub const PURPLE: Color = Color::from_rgb8(0x7c, 0x5c, 0xff);
pub const WHITE: Color = Color::WHITE;
pub const BLACK: Color = Color::BLACK;
pub const WHITE_06: Color = Color::from_rgba8(255, 255, 255, 0.06);
pub const WHITE_08: Color = Color::from_rgba8(255, 255, 255, 0.08);
pub const WHITE_12: Color = Color::from_rgba8(255, 255, 255, 0.12);
pub const WHITE_16: Color = Color::from_rgba8(255, 255, 255, 0.16);
pub const LINE: Color = Color::from_rgba8(255, 255, 255, 0.10);
pub const AMBER: Color = Color::from_rgb8(0xfc, 0xd3, 0x4d);
pub const BAR: Color = Color::from_rgba8(0, 0, 0, 0.55);
pub const UI: Font = Font::with_name("Segoe UI");

pub fn strong() -> Font {
    Font { weight: Weight::Semibold, ..UI }
}

pub fn mica(_theme: &Theme) -> container::Style {
    let gradient = iced::gradient::Linear::new(2.4)
        .add_stop(0.0, Color::from_rgba(1.0, 0.31, 0.55, 0.34))
        .add_stop(0.38, Color::from_rgb8(0x0b, 0x0b, 0x0e))
        .add_stop(1.0, Color::from_rgba(0.49, 0.36, 1.0, 0.28));
    container::Style {
        text_color: Some(FG),
        background: Some(Background::Gradient(gradient.into())),
        ..container::Style::default()
    }
}

pub fn fill(color: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: border::rounded(radius),
        text_color: Some(FG),
        ..container::Style::default()
    }
}

pub fn line(_theme: &Theme) -> container::Style {
    container::Style { background: Some(Background::Color(LINE)), ..container::Style::default() }
}

pub fn row_on(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(WHITE_08)),
        border: border::rounded(12.0),
        ..container::Style::default()
    }
}

pub fn row_off(_theme: &Theme) -> container::Style {
    container::Style { border: border::rounded(12.0), ..container::Style::default() }
}

fn button_style(background: Option<Color>, text_color: Color, radius: f32, hover: Option<Color>) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let color = match status {
            button::Status::Hovered | button::Status::Pressed => hover.or(background),
            button::Status::Disabled => background.map(|color| Color { a: color.a * 0.4, ..color }),
            _ => background,
        };
        button::Style {
            background: color.map(Background::Color),
            text_color,
            border: border::rounded(radius),
            ..button::Style::default()
        }
    }
}

pub fn chip_on(theme: &Theme, status: button::Status) -> button::Style {
    button_style(Some(WHITE), BLACK, 999.0, Some(Color::from_rgb8(0xe8, 0xe8, 0xea)))(theme, status)
}

pub fn chip_off(theme: &Theme, status: button::Status) -> button::Style {
    button_style(Some(WHITE_08), Color::from_rgba8(255, 255, 255, 0.78), 999.0, Some(WHITE_12))(theme, status)
}

pub fn chip_pink(theme: &Theme, status: button::Status) -> button::Style {
    button_style(Some(PINK), BLACK, 999.0, Some(PINK_SOFT))(theme, status)
}

pub fn nav_on(theme: &Theme, status: button::Status) -> button::Style {
    button_style(Some(WHITE_12), FG, 12.0, Some(WHITE_16))(theme, status)
}

pub fn nav_off(theme: &Theme, status: button::Status) -> button::Style {
    button_style(None, Color::from_rgba8(255, 255, 255, 0.72), 12.0, Some(WHITE_06))(theme, status)
}

pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    button_style(None, FG, 12.0, Some(WHITE_08))(theme, status)
}

pub fn play(theme: &Theme, status: button::Status) -> button::Style {
    button_style(Some(WHITE), BLACK, 999.0, Some(Color::from_rgb8(0xe7, 0xe7, 0xe9)))(theme, status)
}

pub fn heart_on(theme: &Theme, status: button::Status) -> button::Style {
    button_style(None, PINK, 999.0, Some(WHITE_08))(theme, status)
}

pub fn heart_off(theme: &Theme, status: button::Status) -> button::Style {
    button_style(None, FAINT, 999.0, Some(WHITE_08))(theme, status)
}

pub fn accent(theme: &Theme, status: button::Status) -> button::Style {
    button_style(None, PINK, 12.0, Some(WHITE_08))(theme, status)
}

pub fn search(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(WHITE_08),
        border: Border { color: LINE, width: 1.0, radius: 999.0.into() },
        icon: FAINT,
        placeholder: FAINT,
        value: FG,
        selection: PINK,
    }
}

pub fn seek(_theme: &Theme, _status: slider::Status) -> slider::Style {
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(PINK), Background::Color(Color::from_rgba8(255, 255, 255, 0.28))),
            width: 5.0,
            border: border::rounded(4.0),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(PINK),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

pub fn volume(_theme: &Theme, _status: slider::Status) -> slider::Style {
    rail(FG, 3.0, 5.0)
}

fn rail(color: Color, width: f32, radius: f32) -> slider::Style {
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(color), Background::Color(WHITE_16)),
            width,
            border: border::rounded(radius),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius },
            background: Background::Color(color),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}
