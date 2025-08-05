//! This module contains serializable font options for cosmic-text.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub enum CursorShape {
    Block,
    #[default]
    Line,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub enum Alignment {
    Left,
    Right,
    #[default]
    Center,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum FontWeight {
    Thin = 100,
    ExtraLight = 200,
    Light = 300,
    #[default]
    Normal = 400,
    Medium = 500,
    SemiBold = 600,
    Bold = 700,
    ExtraBold = 800,
    Black = 900,
}

#[derive(Clone, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub enum FontFamily {
    /// The name of a font family of choice.
    ///
    /// This must be a *Typographic Family* (ID 16) or a *Family Name* (ID 1) in terms of TrueType.
    /// Meaning you have to pass a family without any additional suffixes like _Bold_, _Italic_,
    /// _Regular_, etc.
    ///
    /// Localized names are allowed.
    Name(String),

    /// Serif fonts represent the formal text style for a script.
    Serif,

    /// Glyphs in sans-serif fonts, as the term is used in CSS, are generally low contrast
    /// and have stroke endings that are plain — without any flaring, cross stroke,
    /// or other ornamentation.
    #[default]
    SansSerif,

    /// Glyphs in cursive fonts generally use a more informal script style,
    /// and the result looks more like handwritten pen or brush writing than printed letterwork.
    Cursive,

    /// Fantasy fonts are primarily decorative or expressive fonts that
    /// contain decorative or expressive representations of characters.
    Fantasy,

    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    Monospace,
}

/// Allows italic or oblique faces to be selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub enum FontStyle {
    #[default]
    /// A face that is neither italic not obliqued.
    Normal,
    /// A form that is generally cursive in nature.
    Italic,
    /// A typically-sloped version of the regular face.
    Oblique,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Hash, Serialize, Deserialize)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}
