use std::env;

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Text,
    Keyword,
    String,
    Comment,
    Number,
    Type,
    Function,
    Variable,
    Operator,
    Punctuation,
    Constant,
    Attribute,
    Property,
    Tag,
    Escape,
}

const SCOPE_COUNT: usize = 15;

impl Scope {
    const fn index(self) -> usize {
        match self {
            Self::Text => 0,
            Self::Keyword => 1,
            Self::String => 2,
            Self::Comment => 3,
            Self::Number => 4,
            Self::Type => 5,
            Self::Function => 6,
            Self::Variable => 7,
            Self::Operator => 8,
            Self::Punctuation => 9,
            Self::Constant => 10,
            Self::Attribute => 11,
            Self::Property => 12,
            Self::Tag => 13,
            Self::Escape => 14,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    TrueColor,
    Ansi256,
    Plain,
}

impl ColorDepth {
    pub fn detect() -> Self {
        Self::from_environment(
            env::var_os("NO_COLOR").is_some(),
            env::var("COLORTERM").ok().as_deref(),
        )
    }

    pub fn from_environment(no_color: bool, colorterm: Option<&str>) -> Self {
        if no_color {
            return Self::Plain;
        }
        match colorterm {
            Some(value)
                if value.eq_ignore_ascii_case("truecolor")
                    || value.eq_ignore_ascii_case("24bit") =>
            {
                Self::TrueColor
            }
            _ => Self::Ansi256,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    name: &'static str,
    depth: ColorDepth,
    palette: Palette,
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    background: Rgb,
    foreground: Rgb,
    line_number: Rgb,
    status_background: Rgb,
    status_foreground: Rgb,
    petal: Rgb,
    scopes: [Rgb; SCOPE_COUNT],
}

impl Theme {
    pub fn sakura(depth: ColorDepth) -> Self {
        Self::from_palette("sakura", depth, sakura_palette())
    }

    pub fn sakura_light(depth: ColorDepth) -> Self {
        Self::from_palette("sakura-light", depth, sakura_light_palette())
    }

    pub fn named(name: &str, depth: ColorDepth) -> Option<Self> {
        match name {
            "sakura" => Some(Self::sakura(depth)),
            "sakura-light" => Some(Self::sakura_light(depth)),
            _ => None,
        }
    }

    fn from_palette(name: &'static str, depth: ColorDepth, palette: Palette) -> Self {
        Self {
            name,
            depth,
            palette,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn depth(&self) -> ColorDepth {
        self.depth
    }

    pub fn background(&self) -> Rgb {
        self.palette.background
    }

    pub fn foreground(&self) -> Rgb {
        self.palette.foreground
    }

    pub fn comment(&self) -> Rgb {
        self.palette.scopes[Scope::Comment.index()]
    }

    pub fn line_number(&self) -> Rgb {
        self.palette.line_number
    }

    pub fn scope_color(&self, scope: Scope) -> Rgb {
        self.palette.scopes[scope.index()]
    }

    pub fn style(&self, scope: Scope) -> Style {
        Style::default()
            .fg(self.color(self.scope_color(scope)))
            .bg(self.color(self.palette.background))
            .add_modifier(modifier(scope))
    }

    pub fn text(&self) -> Style {
        self.style(Scope::Text)
    }

    pub fn line_number_style(&self) -> Style {
        Style::default()
            .fg(self.color(self.palette.line_number))
            .bg(self.color(self.palette.background))
    }

    pub fn status_bar(&self) -> Style {
        Style::default()
            .fg(self.color(self.palette.status_foreground))
            .bg(self.color(self.palette.status_background))
            .add_modifier(Modifier::BOLD)
    }

    pub fn selection(&self) -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }

    pub fn match_highlight(&self) -> Style {
        Style::default().add_modifier(Modifier::UNDERLINED)
    }

    pub fn cursor(&self) -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }

    pub fn background_style(&self) -> Style {
        Style::default().bg(self.color(self.palette.background))
    }

    pub fn petal_style(&self) -> Style {
        Style::default()
            .fg(self.color(self.palette.petal))
            .bg(self.color(self.palette.background))
    }

    fn color(&self, rgb: Rgb) -> Color {
        match self.depth {
            ColorDepth::TrueColor => Color::Rgb(rgb.red, rgb.green, rgb.blue),
            ColorDepth::Ansi256 => Color::Indexed(ansi256(rgb)),
            ColorDepth::Plain => Color::Reset,
        }
    }
}

fn modifier(scope: Scope) -> Modifier {
    match scope {
        Scope::Keyword | Scope::Type | Scope::Function => Modifier::BOLD,
        Scope::Comment => Modifier::DIM,
        _ => Modifier::empty(),
    }
}

fn sakura_palette() -> Palette {
    let background = rgb(0x1e, 0x16, 0x20);
    let foreground = rgb(0xf2, 0xe6, 0xec);
    let mute = rgb(0x7a, 0x6a, 0x76);
    let sakura = rgb(0xf4, 0xa6, 0xc0);
    let deep_pink = rgb(0xe0, 0x67, 0x8f);
    let petal = rgb(0xfb, 0xd5, 0xe1);
    let leaf = rgb(0x8f, 0xbf, 0x9f);
    let dusk = rgb(0x8a, 0xa4, 0xd6);
    Palette {
        background,
        foreground,
        line_number: mute,
        status_background: deep_pink,
        status_foreground: background,
        petal,
        scopes: [
            foreground, sakura, leaf, mute, petal, dusk, dusk, foreground, sakura, foreground,
            petal, sakura, dusk, sakura, deep_pink,
        ],
    }
}

fn sakura_light_palette() -> Palette {
    let background = rgb(0xfd, 0xf6, 0xf8);
    let foreground = rgb(0x2a, 0x1f, 0x24);
    let mute = rgb(0x7d, 0x66, 0x70);
    let deep_pink = rgb(0xb2, 0x3a, 0x63);
    let leaf = rgb(0x2f, 0x6b, 0x48);
    let dusk = rgb(0x3f, 0x5f, 0x9e);
    let rose = rgb(0xa0, 0x43, 0x6a);
    Palette {
        background,
        foreground,
        line_number: mute,
        status_background: deep_pink,
        status_foreground: background,
        petal: rgb(0xdf, 0x9a, 0xb2),
        scopes: [
            foreground, deep_pink, leaf, mute, rose, dusk, dusk, foreground, deep_pink, foreground,
            rose, deep_pink, dusk, deep_pink, rose,
        ],
    }
}

pub fn contrast_ratio(first: Rgb, second: Rgb) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    let lighter = first.max(second);
    let darker = first.min(second);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Rgb) -> f64 {
    0.2126 * linearize(color.red) + 0.7152 * linearize(color.green) + 0.0722 * linearize(color.blue)
}

fn linearize(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn ansi256(color: Rgb) -> u8 {
    let red = cube_level(color.red);
    let green = cube_level(color.green);
    let blue = cube_level(color.blue);
    let cube = Rgb {
        red: cube_value(red),
        green: cube_value(green),
        blue: cube_value(blue),
    };
    let sum = u16::from(color.red) + u16::from(color.green) + u16::from(color.blue);
    let average = u8::try_from(sum / 3).unwrap_or(u8::MAX);
    let gray_level = if average > 238 {
        23
    } else {
        average.saturating_sub(3) / 10
    };
    let gray_value = 8 + 10 * gray_level;
    let gray = rgb(gray_value, gray_value, gray_value);
    if distance(color, cube) <= distance(color, gray) {
        16 + 36 * red + 6 * green + blue
    } else {
        232 + gray_level
    }
}

fn cube_level(channel: u8) -> u8 {
    if channel < 48 {
        0
    } else if channel < 115 {
        1
    } else {
        (channel - 35) / 40
    }
}

fn cube_value(level: u8) -> u8 {
    if level == 0 { 0 } else { 55 + level * 40 }
}

fn distance(first: Rgb, second: Rgb) -> u32 {
    let red = u32::from(first.red.abs_diff(second.red));
    let green = u32::from(first.green.abs_diff(second.green));
    let blue = u32::from(first.blue.abs_diff(second.blue));
    red * red + green * green + blue * blue
}

#[cfg(test)]
mod tests {
    use super::{ColorDepth, Rgb, Scope, Theme, ansi256, contrast_ratio, rgb};

    #[test]
    fn text_meets_contrast_requirement() {
        for theme in [
            Theme::sakura(ColorDepth::TrueColor),
            Theme::sakura_light(ColorDepth::TrueColor),
        ] {
            let ratio = contrast_ratio(theme.foreground(), theme.background());
            assert!(ratio >= 4.5, "{} text contrast {ratio}", theme.name());
        }
    }

    #[test]
    fn comments_and_line_numbers_meet_contrast_requirement() {
        for theme in [
            Theme::sakura(ColorDepth::TrueColor),
            Theme::sakura_light(ColorDepth::TrueColor),
        ] {
            let comment = contrast_ratio(theme.comment(), theme.background());
            let line_number = contrast_ratio(theme.line_number(), theme.background());
            assert!(
                comment >= 3.0,
                "{} comment contrast {comment}",
                theme.name()
            );
            assert!(
                line_number >= 3.0,
                "{} line number contrast {line_number}",
                theme.name()
            );
        }
    }

    #[test]
    fn petal_style_contrasts_with_background() {
        for theme in [
            Theme::sakura(ColorDepth::TrueColor),
            Theme::sakura_light(ColorDepth::TrueColor),
        ] {
            let style = theme.petal_style();
            assert_ne!(style.fg, style.bg, "{} petal", theme.name());
        }
    }

    #[test]
    fn depth_detection_honors_no_color_and_colorterm() {
        assert_eq!(
            ColorDepth::from_environment(true, Some("truecolor")),
            ColorDepth::Plain
        );
        assert_eq!(
            ColorDepth::from_environment(false, Some("truecolor")),
            ColorDepth::TrueColor
        );
        assert_eq!(
            ColorDepth::from_environment(false, Some("24bit")),
            ColorDepth::TrueColor
        );
        assert_eq!(
            ColorDepth::from_environment(false, Some("256")),
            ColorDepth::Ansi256
        );
        assert_eq!(
            ColorDepth::from_environment(false, None),
            ColorDepth::Ansi256
        );
    }

    #[test]
    fn truecolor_uses_rgb_values() {
        let theme = Theme::sakura(ColorDepth::TrueColor);
        let style = theme.style(Scope::Keyword);
        assert_eq!(style.fg, Some(ratatui::style::Color::Rgb(0xf4, 0xa6, 0xc0)));
    }

    #[test]
    fn ansi256_uses_indexed_values() {
        let theme = Theme::sakura(ColorDepth::Ansi256);
        let style = theme.style(Scope::Keyword);
        assert!(matches!(style.fg, Some(ratatui::style::Color::Indexed(_))));
        assert_eq!(ansi256(rgb(0, 0, 0)), 16);
        assert_eq!(ansi256(rgb(255, 255, 255)), 231);
        assert_eq!(ansi256(rgb(128, 128, 128)), 244);
    }

    #[test]
    fn plain_uses_reset_colors() {
        for theme in [
            Theme::sakura(ColorDepth::Plain),
            Theme::sakura_light(ColorDepth::Plain),
        ] {
            for scope in [
                Scope::Text,
                Scope::Keyword,
                Scope::String,
                Scope::Comment,
                Scope::Number,
                Scope::Type,
                Scope::Function,
                Scope::Variable,
                Scope::Operator,
                Scope::Punctuation,
                Scope::Constant,
                Scope::Attribute,
                Scope::Property,
                Scope::Tag,
                Scope::Escape,
            ] {
                let style = theme.style(scope);
                assert_eq!(style.fg, Some(ratatui::style::Color::Reset));
                assert_eq!(style.bg, Some(ratatui::style::Color::Reset));
            }
        }
    }

    #[test]
    fn named_themes_resolve() {
        assert!(Theme::named("sakura", ColorDepth::TrueColor).is_some());
        assert!(Theme::named("sakura-light", ColorDepth::TrueColor).is_some());
        assert!(Theme::named("unknown", ColorDepth::TrueColor).is_none());
    }

    #[test]
    fn contrast_ratio_of_black_and_white_is_maximum() {
        let ratio = contrast_ratio(
            Rgb {
                red: 0,
                green: 0,
                blue: 0,
            },
            Rgb {
                red: 255,
                green: 255,
                blue: 255,
            },
        );
        assert!((ratio - 21.0).abs() < 0.01);
    }
}
