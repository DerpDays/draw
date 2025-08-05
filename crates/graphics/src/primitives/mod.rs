mod ellipse;
mod line;
mod pen;
mod quad;
mod rectangle;
mod svg;
mod text;
mod triangle;

use euclid::default::Box2D;
use serde::{Deserialize, Serialize};

use crate::{ApplyCoordinates, Drawable, Mesh, Systems, Vertex};

pub use ellipse::{Ellipse, Options as EllipseOptions};
pub use line::{Line, Options as LineOptions};
pub use pen::{Options as PenOptions, Pen};
pub use quad::{Options as QuadOptions, Quad, QuadPoints};
pub use rectangle::{Options as RectangleOptions, Rectangle};
pub use svg::{Options as SvgOptions, Svg};
pub use text::{Options as TextOptions, Text};
pub use triangle::{Options as TriangleOptions, Triangle};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Primitive<C: ApplyCoordinates + Clone> {
    Ellipse(Ellipse<C>),
    Line(Line<C>),
    Pen(Pen<C>),
    Rectangle(Rectangle<C>),
    Quad(Quad<C>),
    Triangle(Triangle<C>),
    Text(Text<C>),
    Svg(Svg<C>),
}

// TODO: (low_priority) use macro to expand

impl<C: ApplyCoordinates + Clone> Drawable<Vertex> for Primitive<C> {
    fn render(&mut self, systems: &mut Systems) -> &Mesh<Vertex> {
        match self {
            Primitive::Ellipse(elem) => elem.render(systems),
            Primitive::Line(elem) => elem.render(systems),
            Primitive::Pen(elem) => elem.render(systems),
            Primitive::Rectangle(elem) => elem.render(systems),
            Primitive::Quad(elem) => elem.render(systems),
            Primitive::Triangle(elem) => elem.render(systems),
            Primitive::Text(elem) => elem.render(systems),
            Primitive::Svg(elem) => elem.render(systems),
        }
    }
    fn bounding_box(&self) -> Box2D<f32> {
        match self {
            Primitive::Ellipse(elem) => elem.bounding_box(),
            Primitive::Line(elem) => elem.bounding_box(),
            Primitive::Pen(elem) => elem.bounding_box(),
            Primitive::Rectangle(elem) => elem.bounding_box(),
            Primitive::Quad(elem) => elem.bounding_box(),
            Primitive::Triangle(elem) => elem.bounding_box(),
            Primitive::Text(elem) => elem.bounding_box(),
            Primitive::Svg(elem) => elem.bounding_box(),
        }
    }
    fn is_dirty(&self) -> bool {
        match self {
            Primitive::Ellipse(elem) => elem.is_dirty(),
            Primitive::Line(elem) => elem.is_dirty(),
            Primitive::Pen(elem) => elem.is_dirty(),
            Primitive::Rectangle(elem) => elem.is_dirty(),
            Primitive::Quad(elem) => elem.is_dirty(),
            Primitive::Triangle(elem) => elem.is_dirty(),
            Primitive::Text(elem) => elem.is_dirty(),
            Primitive::Svg(elem) => elem.is_dirty(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct Rounding {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}
impl Rounding {
    pub fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_left: value,
            bottom_right: value,
        }
    }
}

impl From<Rounding> for lyon::path::builder::BorderRadii {
    fn from(value: Rounding) -> Self {
        Self {
            top_left: value.top_left,
            top_right: value.top_right,
            bottom_left: value.bottom_left,
            bottom_right: value.bottom_right,
        }
    }
}
