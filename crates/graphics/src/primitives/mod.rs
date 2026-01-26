use std::{any::Any, fmt::Debug, sync::Arc};

use color::{AlphaColor, Srgb};
use euclid::default::{Point2D, Size2D, Vector2D};

use crate::{BasicColor, LineCap, Rounding};
pub mod text;

#[derive(Debug)]
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

    Custom(Box<dyn CustomPrimitive>),
}
impl Clone for Primitive {
    fn clone(&self) -> Self {
        match self {
            Primitive::Custom(c) => Primitive::Custom(c.clone_box()),
            other => other.clone(), // works for the concrete variants
        }
    }
}

impl PartialEq for Primitive {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Primitive::Ellipse(a), Primitive::Ellipse(b)) => a == b,
            (Primitive::Line(a), Primitive::Line(b)) => a == b,
            (Primitive::CubicBezier(a), Primitive::CubicBezier(b)) => a == b,
            (Primitive::Pen(a), Primitive::Pen(b)) => a == b,
            (Primitive::Quad(a), Primitive::Quad(b)) => a == b,
            (Primitive::Rectangle(a), Primitive::Rectangle(b)) => a == b,
            (Primitive::Svg(a), Primitive::Svg(b)) => a == b,
            (Primitive::Text(a), Primitive::Text(b)) => a == b,
            (Primitive::Triangle(a), Primitive::Triangle(b)) => a == b,
            (Primitive::Custom(a), Primitive::Custom(b)) => a.eq_box(&**b),
            _ => false,
        }
    }
}

pub trait CustomPrimitiveImpl {}
pub trait CustomPrimitive: Any + std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn CustomPrimitive>;
    fn eq_box(&self, other: &dyn CustomPrimitive) -> bool;
}
impl<T> CustomPrimitive for T
where
    T: CustomPrimitiveImpl + Any + Clone + Debug + PartialEq + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn CustomPrimitive> {
        Box::new(self.clone())
    }

    fn eq_box(&self, other: &dyn CustomPrimitive) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }
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

/// A non-regular 4 point quadrilateral
///
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

/// A basic rectangle.
///
/// Size is includes the stroke width (like border-box), meaning that the stroke is included in the given size of the
/// rectangle. If the stroke width is greater than the size, only the visible part of the stroke
/// will be rendered.
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

    pub text: String,
    pub color: AlphaColor<Srgb>,
    pub text_layout: TextLayoutOptions,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextLayoutOptions {
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

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AvailableSpace {
    Definite(f32),
    MinContent,
    MaxContent,
}
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextMeasure {
    pub max_width: Option<f32>,
    pub available_space_width: AvailableSpace,

    pub text: String,
    pub text_layout: TextLayoutOptions,
}
