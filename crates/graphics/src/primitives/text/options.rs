use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct FontWidth(pub f32);

impl From<FontWidth> for parley::FontWidth {
    fn from(value: FontWidth) -> parley::FontWidth {
        Self::from_ratio(value.0)
    }
}
impl From<parley::FontWidth> for FontWidth {
    fn from(value: parley::FontWidth) -> FontWidth {
        FontWidth(value.ratio())
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct FontWeight(pub f32);

impl From<FontWeight> for parley::FontWeight {
    fn from(value: FontWeight) -> parley::FontWeight {
        Self::new(value.0)
    }
}
impl From<parley::FontWeight> for FontWeight {
    fn from(value: parley::FontWeight) -> FontWeight {
        FontWeight(value.value())
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique(Option<f32>),
}

impl From<FontStyle> for parley::FontStyle {
    fn from(value: FontStyle) -> parley::FontStyle {
        match value {
            FontStyle::Normal => parley::FontStyle::Normal,
            FontStyle::Italic => parley::FontStyle::Italic,
            FontStyle::Oblique(v) => parley::FontStyle::Oblique(v),
        }
    }
}
impl From<parley::FontStyle> for FontStyle {
    fn from(value: parley::FontStyle) -> FontStyle {
        match value {
            parley::FontStyle::Normal => FontStyle::Normal,
            parley::FontStyle::Italic => FontStyle::Italic,
            parley::FontStyle::Oblique(v) => FontStyle::Oblique(v),
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum LineHeight {
    MetricsRelative(f32),
    FontSizeRelative(f32),
    Absolute(f32),
}

impl From<LineHeight> for parley::LineHeight {
    fn from(value: LineHeight) -> parley::LineHeight {
        match value {
            LineHeight::MetricsRelative(v) => parley::LineHeight::MetricsRelative(v),
            LineHeight::FontSizeRelative(v) => parley::LineHeight::FontSizeRelative(v),
            LineHeight::Absolute(v) => parley::LineHeight::Absolute(v),
        }
    }
}
impl From<parley::LineHeight> for LineHeight {
    fn from(value: parley::LineHeight) -> LineHeight {
        match value {
            parley::LineHeight::MetricsRelative(v) => LineHeight::MetricsRelative(v),
            parley::LineHeight::FontSizeRelative(v) => LineHeight::FontSizeRelative(v),
            parley::LineHeight::Absolute(v) => LineHeight::Absolute(v),
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum OverflowWrap {
    Normal = 0,
    Anywhere = 1,
    BreakWord = 2,
}

impl From<OverflowWrap> for parley::OverflowWrap {
    fn from(value: OverflowWrap) -> parley::OverflowWrap {
        match value {
            OverflowWrap::Normal => parley::OverflowWrap::Normal,
            OverflowWrap::Anywhere => parley::OverflowWrap::Anywhere,
            OverflowWrap::BreakWord => parley::OverflowWrap::BreakWord,
        }
    }
}
impl From<parley::OverflowWrap> for OverflowWrap {
    fn from(value: parley::OverflowWrap) -> OverflowWrap {
        match value {
            parley::OverflowWrap::Normal => OverflowWrap::Normal,
            parley::OverflowWrap::Anywhere => OverflowWrap::Anywhere,
            parley::OverflowWrap::BreakWord => OverflowWrap::BreakWord,
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum WordBreakStrength {
    Normal = 0,
    BreakAll = 1,
    KeepAll = 2,
}

impl From<WordBreakStrength> for parley::WordBreakStrength {
    fn from(value: WordBreakStrength) -> parley::WordBreakStrength {
        match value {
            WordBreakStrength::Normal => parley::WordBreakStrength::Normal,
            WordBreakStrength::BreakAll => parley::WordBreakStrength::BreakAll,
            WordBreakStrength::KeepAll => parley::WordBreakStrength::KeepAll,
        }
    }
}
impl From<parley::WordBreakStrength> for WordBreakStrength {
    fn from(value: parley::WordBreakStrength) -> WordBreakStrength {
        match value {
            parley::WordBreakStrength::Normal => WordBreakStrength::Normal,
            parley::WordBreakStrength::BreakAll => WordBreakStrength::BreakAll,
            parley::WordBreakStrength::KeepAll => WordBreakStrength::KeepAll,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum WhiteSpaceCollapse {
    Collapse,
    Preserve,
}

impl From<WhiteSpaceCollapse> for parley::WhiteSpaceCollapse {
    fn from(value: WhiteSpaceCollapse) -> parley::WhiteSpaceCollapse {
        match value {
            WhiteSpaceCollapse::Preserve => parley::WhiteSpaceCollapse::Preserve,
            WhiteSpaceCollapse::Collapse => parley::WhiteSpaceCollapse::Collapse,
        }
    }
}
impl From<parley::WhiteSpaceCollapse> for WhiteSpaceCollapse {
    fn from(value: parley::WhiteSpaceCollapse) -> WhiteSpaceCollapse {
        match value {
            parley::WhiteSpaceCollapse::Preserve => WhiteSpaceCollapse::Preserve,
            parley::WhiteSpaceCollapse::Collapse => WhiteSpaceCollapse::Collapse,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FontFamily {
    Named(String),
    Generic(GenericFamily),
}

impl<'a> From<FontFamily> for parley::FontFamily<'a> {
    fn from(value: FontFamily) -> parley::FontFamily<'a> {
        match value {
            FontFamily::Named(name) => parley::FontFamily::Named(std::borrow::Cow::Owned(name)),
            FontFamily::Generic(v) => parley::FontFamily::Generic(v.into()),
        }
    }
}
impl<'a> From<parley::FontFamily<'a>> for FontFamily {
    fn from(value: parley::FontFamily) -> FontFamily {
        match value {
            parley::FontFamily::Named(name) => FontFamily::Named(name.to_string()),
            parley::FontFamily::Generic(v) => FontFamily::Generic(v.into()),
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum GenericFamily {
    Serif = 0,
    SansSerif = 1,
    Monospace = 2,
    Cursive = 3,
    Fantasy = 4,
    SystemUi = 5,
    UiSerif = 6,
    UiSansSerif = 7,
    UiMonospace = 8,
    UiRounded = 9,
    Emoji = 10,
    Math = 11,
    FangSong = 12,
}

impl From<GenericFamily> for parley::GenericFamily {
    fn from(value: GenericFamily) -> parley::GenericFamily {
        match value {
            GenericFamily::Serif => parley::GenericFamily::Serif,
            GenericFamily::SansSerif => parley::GenericFamily::SansSerif,
            GenericFamily::Monospace => parley::GenericFamily::Monospace,
            GenericFamily::Cursive => parley::GenericFamily::Cursive,
            GenericFamily::Fantasy => parley::GenericFamily::Fantasy,
            GenericFamily::SystemUi => parley::GenericFamily::SystemUi,
            GenericFamily::UiSerif => parley::GenericFamily::UiSerif,
            GenericFamily::UiSansSerif => parley::GenericFamily::UiSansSerif,
            GenericFamily::UiMonospace => parley::GenericFamily::UiMonospace,
            GenericFamily::UiRounded => parley::GenericFamily::UiRounded,
            GenericFamily::Emoji => parley::GenericFamily::Emoji,
            GenericFamily::Math => parley::GenericFamily::Math,
            GenericFamily::FangSong => parley::GenericFamily::FangSong,
        }
    }
}
impl From<parley::GenericFamily> for GenericFamily {
    fn from(value: parley::GenericFamily) -> GenericFamily {
        match value {
            parley::GenericFamily::Serif => GenericFamily::Serif,
            parley::GenericFamily::SansSerif => GenericFamily::SansSerif,
            parley::GenericFamily::Monospace => GenericFamily::Monospace,
            parley::GenericFamily::Cursive => GenericFamily::Cursive,
            parley::GenericFamily::Fantasy => GenericFamily::Fantasy,
            parley::GenericFamily::SystemUi => GenericFamily::SystemUi,
            parley::GenericFamily::UiSerif => GenericFamily::UiSerif,
            parley::GenericFamily::UiSansSerif => GenericFamily::UiSansSerif,
            parley::GenericFamily::UiMonospace => GenericFamily::UiMonospace,
            parley::GenericFamily::UiRounded => GenericFamily::UiRounded,
            parley::GenericFamily::Emoji => GenericFamily::Emoji,
            parley::GenericFamily::Math => GenericFamily::Math,
            parley::GenericFamily::FangSong => GenericFamily::FangSong,
        }
    }
}
