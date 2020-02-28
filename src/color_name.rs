use std::str::FromStr;

/// List of Color Names. Refer to [this page](https://imagemagick.org/script/color.php).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorName {
    /// #FFFFFF
    White,
    /// #000000
    Black,
    /// #FF0000
    Red,
    /// #00FF00
    Green,
    /// #0000FF
    Blue,
    /// #FFFF00
    Yellow,
    /// #00FFFF
    Cyan,
    /// #FF00FF
    Magenta,
}

impl ColorName {
    /// Get the static string slice of this color name.
    pub fn as_str(self) -> &'static str {
        match self {
            ColorName::White => "white",
            ColorName::Black => "black",
            ColorName::Red => "red",
            ColorName::Green => "green",
            ColorName::Blue => "blue",
            ColorName::Yellow => "yellow",
            ColorName::Cyan => "cyan",
            ColorName::Magenta => "magenta",
        }
    }

    /// Get the static string slice of this color name.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Option<ColorName> {
        let s = s.as_ref().to_lowercase();

        match s.as_str() {
            "white" => Some(ColorName::White),
            "black" => Some(ColorName::Black),
            "red" => Some(ColorName::Red),
            "green" => Some(ColorName::Green),
            "blue" => Some(ColorName::Blue),
            "yellow" => Some(ColorName::Yellow),
            "cyan" => Some(ColorName::Cyan),
            "magenta" => Some(ColorName::Magenta),
            _ => None,
        }
    }
}

impl FromStr for ColorName {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ColorName::from_str(s).ok_or_else(|| ())
    }
}
