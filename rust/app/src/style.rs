use std::sync::atomic::{AtomicU8, Ordering};

use crate::model::Look;
use iced::font::Weight;
use iced::widget::{button, container, slider, text_input};
use iced::{Background, Border, Color, Font, Theme, border};

struct Paint {
    fg: Color,
    muted: Color,
    faint: Color,
    nav: Color,
    accent: Color,
    accent_soft: Color,
    on_accent: Color,
    purple: Color,
    wash: Color,
    wash_mid: Color,
    wash_strong: Color,
    wash_hover: Color,
    line: Color,
    amber: Color,
    bar: Color,
    surface: Color,
    play_bg: Color,
    play_fg: Color,
    play_hover: Color,
    chip_bg: Color,
    chip_fg: Color,
    chip_hover: Color,
    night: bool,
}

const UI: Font = Font::with_name("Segoe UI");

static LOOK: AtomicU8 = AtomicU8::new(0);

pub fn apply(look: Look) {
    LOOK.store(code(look), Ordering::Relaxed);
}

pub fn strong() -> Font {
    Font { weight: Weight::Semibold, ..UI }
}

pub fn fg() -> Color { paint().fg }
pub fn muted() -> Color { paint().muted }
pub fn faint() -> Color { paint().faint }
pub fn highlight() -> Color { paint().accent }
pub fn on_accent() -> Color { paint().on_accent }
pub fn purple() -> Color { paint().purple }
pub fn play_icon() -> Color { paint().play_fg }
pub fn amber() -> Color { paint().amber }
pub fn bar() -> Color { paint().bar }
pub fn surface() -> Color { paint().surface }

pub fn swatch(look: Look) -> Color {
    match look {
        Look::Night => Color::from_rgb8(0x0b, 0x0b, 0x0e),
        Look::Light => Color::from_rgb8(0xf6, 0xf3, 0xf5),
        Look::Purple => Color::from_rgb8(0x7c, 0x5c, 0xff),
    }
}

pub fn mica(_theme: &Theme) -> container::Style {
    let colors = paint();
    let gradient = if colors.night {
        iced::gradient::Linear::new(2.4)
            .add_stop(0.0, Color::from_rgba(1.0, 0.31, 0.55, 0.34))
            .add_stop(0.38, Color::from_rgb8(0x0b, 0x0b, 0x0e))
            .add_stop(1.0, Color::from_rgba(0.49, 0.36, 1.0, 0.28))
    } else if current() == Look::Purple {
        iced::gradient::Linear::new(2.4)
            .add_stop(0.0, Color::from_rgba(0.49, 0.36, 1.0, 0.46))
            .add_stop(0.4, Color::from_rgb8(0x0c, 0x0a, 0x14))
            .add_stop(1.0, Color::from_rgba(0.49, 0.36, 1.0, 0.32))
    } else {
        iced::gradient::Linear::new(2.4)
            .add_stop(0.0, Color::from_rgba(1.0, 0.31, 0.55, 0.20))
            .add_stop(0.42, Color::from_rgb8(0xf6, 0xf3, 0xf5))
            .add_stop(1.0, Color::from_rgba(0.49, 0.36, 1.0, 0.14))
    };
    container::Style {
        text_color: Some(colors.fg),
        background: Some(Background::Gradient(gradient.into())),
        ..container::Style::default()
    }
}

pub fn dot(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: Border { color: Color::from_rgba(0.0, 0.0, 0.0, 0.22), width: 1.0, radius: 7.0.into() },
        ..container::Style::default()
    }
}

pub fn fill(color: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    let fg = paint().fg;
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: border::rounded(radius),
        text_color: Some(fg),
        ..container::Style::default()
    }
}

pub fn line(_theme: &Theme) -> container::Style {
    container::Style { background: Some(Background::Color(paint().line)), ..container::Style::default() }
}

pub fn row_on(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(paint().wash_mid)),
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
    let colors = paint();
    button_style(Some(colors.chip_bg), colors.chip_fg, 999.0, Some(colors.chip_hover))(theme, status)
}

pub fn chip_off(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(Some(colors.wash_mid), colors.nav, 999.0, Some(colors.wash_strong))(theme, status)
}

pub fn chip_pink(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(Some(colors.accent), colors.on_accent, 999.0, Some(colors.accent_soft))(theme, status)
}

pub fn nav_on(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(Some(colors.wash_strong), colors.fg, 12.0, Some(colors.wash_hover))(theme, status)
}

pub fn nav_off(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(None, colors.nav, 12.0, Some(colors.wash))(theme, status)
}

pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(None, colors.fg, 12.0, Some(colors.wash_mid))(theme, status)
}

pub fn play(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(Some(colors.play_bg), colors.play_fg, 999.0, Some(colors.play_hover))(theme, status)
}

pub fn heart_on(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(None, colors.accent, 999.0, Some(colors.wash_mid))(theme, status)
}

pub fn heart_off(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(None, colors.faint, 999.0, Some(colors.wash_mid))(theme, status)
}

pub fn accent(theme: &Theme, status: button::Status) -> button::Style {
    let colors = paint();
    button_style(None, colors.accent, 12.0, Some(colors.wash_mid))(theme, status)
}

pub fn search(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let colors = paint();
    text_input::Style {
        background: Background::Color(colors.wash_mid),
        border: Border { color: colors.line, width: 1.0, radius: 999.0.into() },
        icon: colors.faint,
        placeholder: colors.faint,
        value: colors.fg,
        selection: colors.accent,
    }
}

pub fn seek(_theme: &Theme, _status: slider::Status) -> slider::Style {
    let colors = paint();
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(colors.accent), Background::Color(colors.wash_hover)),
            width: 5.0,
            border: border::rounded(4.0),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(colors.accent),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

pub fn volume(_theme: &Theme, _status: slider::Status) -> slider::Style {
    let colors = paint();
    rail(colors.fg, colors.wash_hover, 3.0, 5.0)
}

fn rail(color: Color, rest: Color, width: f32, radius: f32) -> slider::Style {
    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(color), Background::Color(rest)),
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

fn current() -> Look {
    match LOOK.load(Ordering::Relaxed) {
        1 => Look::Light,
        2 => Look::Purple,
        _ => Look::Night,
    }
}

fn code(look: Look) -> u8 {
    match look {
        Look::Night => 0,
        Look::Light => 1,
        Look::Purple => 2,
    }
}

fn paint() -> Paint {
    match current() {
        Look::Light => light_paint(),
        Look::Purple => purple_paint(),
        Look::Night => night_paint(),
    }
}

fn night_paint() -> Paint {
    Paint {
        fg: Color::from_rgb8(0xf5, 0xf5, 0xf6),
        muted: Color::from_rgba8(255, 255, 255, 0.55),
        faint: Color::from_rgba8(255, 255, 255, 0.38),
        nav: Color::from_rgba8(255, 255, 255, 0.72),
        accent: Color::from_rgb8(0xff, 0x4f, 0x8b),
        accent_soft: Color::from_rgba8(0xff, 0x4f, 0x8b, 0.85),
        on_accent: Color::from_rgb8(0x1a, 0x0b, 0x12),
        purple: Color::from_rgb8(0x7c, 0x5c, 0xff),
        wash: Color::from_rgba8(255, 255, 255, 0.06),
        wash_mid: Color::from_rgba8(255, 255, 255, 0.08),
        wash_strong: Color::from_rgba8(255, 255, 255, 0.12),
        wash_hover: Color::from_rgba8(255, 255, 255, 0.16),
        line: Color::from_rgba8(255, 255, 255, 0.10),
        amber: Color::from_rgb8(0xfc, 0xd3, 0x4d),
        bar: Color::from_rgba8(0, 0, 0, 0.55),
        surface: Color::from_rgb8(0x2c, 0x1c, 0x28),
        play_bg: Color::WHITE,
        play_fg: Color::BLACK,
        play_hover: Color::from_rgb8(0xe7, 0xe7, 0xe9),
        chip_bg: Color::WHITE,
        chip_fg: Color::BLACK,
        chip_hover: Color::from_rgb8(0xe8, 0xe8, 0xea),
        night: true,
    }
}

fn light_paint() -> Paint {
    let ink = Color::from_rgb8(0x1a, 0x0b, 0x12);
    Paint {
        fg: ink,
        muted: Color::from_rgba8(26, 11, 18, 0.62),
        faint: Color::from_rgba8(26, 11, 18, 0.42),
        nav: Color::from_rgba8(26, 11, 18, 0.72),
        accent: Color::from_rgb8(0xff, 0x4f, 0x8b),
        accent_soft: Color::from_rgba8(0xff, 0x4f, 0x8b, 0.85),
        on_accent: ink,
        purple: Color::from_rgb8(0x7c, 0x5c, 0xff),
        wash: Color::from_rgba8(26, 11, 18, 0.05),
        wash_mid: Color::from_rgba8(26, 11, 18, 0.07),
        wash_strong: Color::from_rgba8(26, 11, 18, 0.10),
        wash_hover: Color::from_rgba8(26, 11, 18, 0.14),
        line: Color::from_rgba8(26, 11, 18, 0.10),
        amber: Color::from_rgb8(0xb4, 0x53, 0x09),
        bar: Color::from_rgba8(255, 255, 255, 0.86),
        surface: Color::from_rgb8(0xff, 0xff, 0xff),
        play_bg: ink,
        play_fg: Color::WHITE,
        play_hover: Color::from_rgb8(0x3d, 0x24, 0x30),
        chip_bg: ink,
        chip_fg: Color::WHITE,
        chip_hover: Color::from_rgb8(0x3d, 0x24, 0x30),
        night: false,
    }
}

fn purple_paint() -> Paint {
    let mut colors = night_paint();
    colors.accent = Color::from_rgb8(0x7c, 0x5c, 0xff);
    colors.accent_soft = Color::from_rgba8(0x7c, 0x5c, 0xff, 0.85);
    colors.on_accent = Color::WHITE;
    colors.surface = Color::from_rgb8(0x1c, 0x16, 0x30);
    colors.night = false;
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_keeps_the_pink_mark_and_the_others_do_not() {
        apply(Look::Night);
        assert_eq!(highlight(), Color::from_rgb8(0xff, 0x4f, 0x8b));
        apply(Look::Purple);
        assert_eq!(highlight(), Color::from_rgb8(0x7c, 0x5c, 0xff));
        apply(Look::Light);
        assert_eq!(fg(), Color::from_rgb8(0x1a, 0x0b, 0x12));
        apply(Look::Night);
    }
}
