mod ellipse;
mod line;
mod pen;
mod quad;
mod rectangle;
mod svg;
mod text;
mod triangle;

use serde::{Deserialize, Serialize};

use crate::{ApplyCoordinates, Drawable, Vertex};

pub use ellipse::{Ellipse, Options as EllipseOptions};
pub use line::{Line, Options as LineOptions};
pub use pen::{Options as PenOptions, Pen};
pub use quad::{Options as QuadOptions, Quad, QuadPoints};
pub use rectangle::{Options as RectangleOptions, Rectangle};
pub use svg::{Options as SvgOptions, Svg};
pub use text::{Options as TextOptions, Text};
pub use triangle::{Options as TriangleOptions, Triangle};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Primitive<C: ApplyCoordinates + Clone> {
    Ellipse(Ellipse<C>),
    Line(Line<C>),
    Pen(Pen<C>),
    Quad(Quad<C>),
    Rectangle(Rectangle<C>),
    Svg(Svg<C>),
    Text(Text<C>),
    Triangle(Triangle<C>),
}

macro_rules! delegate_primitive {
    (
        $( $variant:ident ),*
    ) => {
        fn render(&mut self, systems: &mut $crate::Systems) -> &$crate::Mesh<Vertex> {
            match self {
                $( Primitive::$variant(w) => w.render(systems), )*
            }
        }
        fn is_dirty(&self) -> bool {
            match self {
                $( Primitive::$variant(w) => w.is_dirty(), )*
            }
        }
        fn bounding_box(&self) -> euclid::default::Box2D<f32> {
            match self {
                $( Primitive::$variant(w) => w.bounding_box(), )*
            }
        }
    };
}

impl<C: ApplyCoordinates + Clone> Drawable<Vertex> for Primitive<C> {
    delegate_primitive!(Ellipse, Line, Pen, Quad, Rectangle, Svg, Text, Triangle);
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
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
