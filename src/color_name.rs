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
    /// #008000
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
    const ALL: [ColorName; 8] = [
        Self::White,
        Self::Black,
        Self::Red,
        Self::Green,
        Self::Blue,
        Self::Yellow,
        Self::Cyan,
        Self::Magenta,
    ];

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

    /// Parse a color name in a case-insensitive way.
    pub fn parse_str<S: AsRef<str>>(s: S) -> Option<Self> {
        let s = s.as_ref();

        Self::ALL.into_iter().find(|name| s.eq_ignore_ascii_case(name.as_str()))
    }
}

impl FromStr for ColorName {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s).ok_or(())
    }
}
