#[cfg(feature = "parley")]
mod parley;

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontWidth(pub f32);
impl FontWidth {
    /// Width that is 50% of normal.
    pub const ULTRA_CONDENSED: Self = Self(0.5);
    /// Width that is 62.5% of normal.
    pub const EXTRA_CONDENSED: Self = Self(0.625);
    /// Width that is 75% of normal.
    pub const CONDENSED: Self = Self(0.75);
    /// Width that is 87.5% of normal.
    pub const SEMI_CONDENSED: Self = Self(0.875);
    /// Width that is 100% of normal. This is the default value.
    pub const NORMAL: Self = Self(1.0);
    /// Width that is 112.5% of normal.
    pub const SEMI_EXPANDED: Self = Self(1.125);
    /// Width that is 125% of normal.
    pub const EXPANDED: Self = Self(1.25);
    /// Width that is 150% of normal.
    pub const EXTRA_EXPANDED: Self = Self(1.5);
    /// Width that is 200% of normal.
    pub const ULTRA_EXPANDED: Self = Self(2.0);
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontWeight(pub f32);
impl FontWeight {
    /// Weight value of 100.
    pub const THIN: Self = Self(100.0);
    /// Weight value of 200.
    pub const EXTRA_LIGHT: Self = Self(200.0);
    /// Weight value of 300.
    pub const LIGHT: Self = Self(300.0);
    /// Weight value of 350.
    pub const SEMI_LIGHT: Self = Self(350.0);
    /// Weight value of 400. This is the default value.
    pub const NORMAL: Self = Self(400.0);
    /// Weight value of 500.
    pub const MEDIUM: Self = Self(500.0);
    /// Weight value of 600.
    pub const SEMI_BOLD: Self = Self(600.0);
    /// Weight value of 700.
    pub const BOLD: Self = Self(700.0);
    /// Weight value of 800.
    pub const EXTRA_BOLD: Self = Self(800.0);
    /// Weight value of 900.
    pub const BLACK: Self = Self(900.0);
    /// Weight value of 950.
    pub const EXTRA_BLACK: Self = Self(950.0);
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique(Option<f32>),
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LineHeight {
    MetricsRelative(f32),
    FontSizeRelative(f32),
    Absolute(f32),
}
impl Default for LineHeight {
    fn default() -> Self {
        Self::MetricsRelative(1.0)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OverflowWrap {
    Normal,
    Anywhere,
    BreakWord,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WordBreakStrength {
    Normal,
    BreakAll,
    KeepAll,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WhiteSpaceCollapse {
    Collapse,
    Preserve,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FontFamily {
    Named(String),
    Generic(GenericFamily),
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GenericFamily {
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
    #[default]
    SystemUi,
    UiSerif,
    UiSansSerif,
    UiMonospace,
    UiRounded,
    Emoji,
    Math,
    FangSong,
}
