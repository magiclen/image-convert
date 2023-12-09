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
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Yellow => "yellow",
            Self::Cyan => "cyan",
            Self::Magenta => "magenta",
        }
    }

    /// Get the static string slice of this color name.
    pub fn parse_str<S: AsRef<str>>(s: S) -> Option<Self> {
        let s = s.as_ref().to_lowercase();

        match s.as_str() {
            "white" => Some(Self::White),
            "black" => Some(Self::Black),
            "red" => Some(Self::Red),
            "green" => Some(Self::Green),
            "blue" => Some(Self::Blue),
            "yellow" => Some(Self::Yellow),
            "cyan" => Some(Self::Cyan),
            "magenta" => Some(Self::Magenta),
            _ => None,
        }
    }
}

impl FromStr for ColorName {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s).ok_or(())
    }
}
