use std::sync::Arc;

use euclid::default::{Point2D, Size2D, Vector2D};

use crate::{BasicColor, LineCap, Rounding};
pub mod text;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Primitive {
    Ellipse(Ellipse),
    Line(Line),
    CubicBezier(CubicBezier),
    Pen(Pen),
    Quad(Quad),
    Rectangle(Rectangle),
    Svg(Svg),
    Text(Text),
    Triangle(Triangle),
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ellipse {
    pub center: Point2D<f32>,
    pub radius: Vector2D<f32>,

    pub color: BasicColor,

    pub stroke_color: BasicColor,
    pub stroke_width: f32,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Line {
    pub origin: Point2D<f32>,
    pub destination: Point2D<f32>,

    pub cap: LineCap,

    pub color: BasicColor,
    pub width: f32,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CubicBezier {
    pub p0: Point2D<f32>,
    pub p1: Point2D<f32>,
    pub p2: Point2D<f32>,
    pub p3: Point2D<f32>,

    pub cap: LineCap,

    pub color: BasicColor,
    pub width: BasicColor,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pen {
    pub points: Vec<Point2D<f32>>,

    pub cap: LineCap,

    pub color: BasicColor,
    pub width: BasicColor,
}

/// p0 --- p1
/// |      |
/// p2 --- p3
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quad {
    pub p0: Point2D<f32>,
    pub p1: Point2D<f32>,
    pub p2: Point2D<f32>,
    pub p3: Point2D<f32>,

    pub rounding: Rounding,

    pub color: BasicColor,

    pub stroke_color: BasicColor,
    pub stroke_width: f32,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rectangle {
    pub origin: Point2D<f32>,
    pub size: Size2D<f32>,

    pub rounding: Rounding,

    pub color: BasicColor,

    pub stroke_color: BasicColor,
    pub stroke_width: f32,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Triangle {
    pub p0: Point2D<f32>,
    pub p1: Point2D<f32>,
    pub p2: Point2D<f32>,

    pub color: BasicColor,

    pub stroke_color: BasicColor,
    pub stroke_width: f32,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Svg {
    pub origin: Point2D<f32>,
    pub size: Size2D<f32>,

    pub data: Arc<[u8]>,

    pub fill_color: Option<BasicColor>,
    pub stroke_color: Option<BasicColor>,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Text {
    pub origin: Point2D<f32>,
    pub size: Size2D<f32>,

    pub text: Arc<str>,
    pub color: Option<BasicColor>,

    pub font_family: text::FontFamily,
    pub font_size: f32,
    pub font_style: text::FontStyle,
    pub font_weight: text::FontWeight,
    pub font_width: text::FontWidth,
    pub line_height: text::LineHeight,
    pub overflow_wrap: text::OverflowWrap,
    pub whitespace_collapse: text::WhiteSpaceCollapse,
    pub word_break_strength: text::WordBreakStrength,
}
