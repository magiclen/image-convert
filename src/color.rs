use std::borrow::Cow;

use crate::ColorName;

/// A color which **MagickWand** can recognize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Color {
    /// A color picked from the list of color names.
    Name(ColorName),
    /// A color made up of red, green and blue.
    Rgb(u8, u8, u8),
    /// A color made up of red, green, blue and alpha.
    Rgba(u8, u8, u8, u8),
    /// Any other color string which **MagickWand** can recognize, such as `hsl(0, 100%, 50%)` or `gray(50%)`.
    Custom(Cow<'static, str>),
}

impl Color {
    /// #FFFFFF
    pub const BLACK: Self = Self::Name(ColorName::Black);
    /// #0000FF
    pub const BLUE: Self = Self::Name(ColorName::Blue);
    /// #00FFFF
    pub const CYAN: Self = Self::Name(ColorName::Cyan);
    /// #00FF00
    pub const GREEN: Self = Self::Name(ColorName::Green);
    /// #FF00FF
    pub const MAGENTA: Self = Self::Name(ColorName::Magenta);
    /// #FF0000
    pub const RED: Self = Self::Name(ColorName::Red);
    /// #000000
    pub const WHITE: Self = Self::Name(ColorName::White);
    /// #FFFF00
    pub const YELLOW: Self = Self::Name(ColorName::Yellow);

    /// Get the color string which **MagickWand** can recognize.
    pub fn to_magick_color(&self) -> Cow<'_, str> {
        match self {
            Self::Name(name) => Cow::Borrowed(name.as_str()),
            Self::Rgb(r, g, b) => Cow::Owned(format!("#{r:02X}{g:02X}{b:02X}")),
            Self::Rgba(r, g, b, a) => Cow::Owned(format!("#{r:02X}{g:02X}{b:02X}{a:02X}")),
            Self::Custom(s) => Cow::Borrowed(s.as_ref()),
        }
    }
}

impl From<ColorName> for Color {
    #[inline]
    fn from(value: ColorName) -> Self {
        Self::Name(value)
    }
}

impl From<&'static str> for Color {
    #[inline]
    fn from(value: &'static str) -> Self {
        Self::Custom(Cow::Borrowed(value))
    }
}

impl From<String> for Color {
    #[inline]
    fn from(value: String) -> Self {
        Self::Custom(Cow::Owned(value))
    }
}
