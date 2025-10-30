#![feature(const_trait_impl)]

use color::{AlphaColor, HueDirection, PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Vector2D};

// pub mod line;
pub mod primitives;
pub use primitives::Primitive;

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseGradient {
    pub stops: Vec<GradientStop<PremulColor<Srgb>>>,

    spread: SpreadMethod,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientStop<T: Copy> {
    pub percent: f32,
    pub color: T,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RadialGradient {
    pub(crate) base: BaseGradient,

    /// Center point of the circle.
    pub(crate) center_point: Point2D<f32>,
    /// Where the gradient starts
    pub(crate) focal_point: Point2D<f32>,
    /// X and Y radius of the gradient.
    pub(crate) radius: Vector2D<f32>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasicLinearGradient {
    pub(crate) start_color: AlphaColor<Srgb>,
    pub(crate) end_color: AlphaColor<Srgb>,

    pub(crate) p1: Point2D<f32>,
    pub(crate) p2: Point2D<f32>,

    pub(crate) spread: SpreadMethod,
}
impl BasicLinearGradient {
    pub const fn new(start_color: AlphaColor<Srgb>, end_color: AlphaColor<Srgb>) -> Self {
        Self {
            start_color,
            end_color,
            p1: Point2D::new(0., 0.),
            p2: Point2D::new(0., 0.),
            spread: SpreadMethod::Pad,
        }
    }
    pub const fn new_with_points(
        start_color: AlphaColor<Srgb>,
        end_color: AlphaColor<Srgb>,
        p1: Point2D<f32>,
        p2: Point2D<f32>,
        spread_method: SpreadMethod,
    ) -> Self {
        Self {
            start_color,
            end_color,
            p1,
            p2,
            spread: spread_method,
        }
    }
    pub const fn update_points(&mut self, p1: Point2D<f32>, p2: Point2D<f32>) {
        self.p1 = p1;
        self.p2 = p2;
    }

    pub const fn as_color(self) -> BasicColor {
        BasicColor::LinearGradient(self)
    }

    pub fn get_point(&self, point: Point2D<f32>) -> AlphaColor<Srgb> {
        // Vector from start to end
        let direction = self.p2 - self.p1;
        // Squared length of the gradient vector
        let length_sq = direction.square_length();

        // Avoid division by zero for degenerate segment
        if length_sq == 0.0 {
            log::warn!("get_point called with zero length");
            log::warn!("p1: {:?}, p2: {:?}", self.p1, self.p2);
            return self.start_color;
        }

        // Vector from p1 to the query point
        let to_point = point - self.p1;

        // Project to_point onto direction vector to find normalized parameter t
        let mut t = (to_point.x * direction.x + to_point.y * direction.y) / length_sq;

        // Apply spread method
        match self.spread {
            SpreadMethod::Pad => {
                t = t.clamp(0., 1.);
            }
            SpreadMethod::Repeat => {
                t = t.fract();
                if t < 0.0 {
                    t += 1.0;
                }
            }
            SpreadMethod::Reflect => {
                t = t.abs() % 2.0;
                if t > 1.0 {
                    t = 2.0 - t;
                }
            }
        };

        self.start_color
            .lerp(self.end_color, t, HueDirection::Shorter)
    }

    pub fn lerp(&self, other: Self, t: f32) -> Self {
        Self {
            start_color: self.start_color.lerp_rect(other.start_color, t),
            end_color: self.end_color.lerp_rect(other.end_color, t),
            p1: self.p1.lerp(other.p1, t),
            p2: self.p2.lerp(other.p2, t),
            spread: if t < 1. { self.spread } else { other.spread },
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BasicColor {
    Solid(AlphaColor<Srgb>),
    LinearGradient(BasicLinearGradient),
}

impl BasicColor {
    pub fn lerp(&self, other: Self, t: f32) -> Self {
        match self {
            BasicColor::Solid(color) => match other {
                BasicColor::Solid(color2) => color.lerp_rect(color2, t).into(),
                BasicColor::LinearGradient(gradient) => BasicLinearGradient {
                    start_color: color.lerp_rect(gradient.start_color, t),
                    end_color: color.lerp_rect(gradient.end_color, t),
                    p1: gradient.p1,
                    p2: gradient.p2,
                    spread: gradient.spread,
                }
                .into(),
            },
            BasicColor::LinearGradient(gradient) => match other {
                BasicColor::Solid(color) => {
                    if t < 1. {
                        BasicLinearGradient {
                            start_color: gradient.start_color.lerp_rect(color, t),
                            end_color: gradient.end_color.lerp_rect(color, t),
                            p1: gradient.p1,
                            p2: gradient.p2,
                            spread: gradient.spread,
                        }
                        .into()
                    } else {
                        other
                    }
                }
                BasicColor::LinearGradient(gradient2) => gradient.lerp(gradient2, t).into(),
            },
        }
    }
}

impl From<AlphaColor<Srgb>> for BasicColor {
    fn from(value: AlphaColor<Srgb>) -> Self {
        Self::Solid(value)
    }
}
impl From<BasicLinearGradient> for BasicColor {
    fn from(value: BasicLinearGradient) -> Self {
        Self::LinearGradient(value)
    }
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComplexColor {
    RadialGradient(RadialGradient),
}

pub const fn make_positive_box(mut area: Box2D<f32>) -> Box2D<f32> {
    if area.min.x > area.max.x {
        std::mem::swap(&mut area.min.x, &mut area.max.x);
    };
    if area.min.y > area.max.y {
        std::mem::swap(&mut area.min.y, &mut area.max.y);
    };
    area
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rounding {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}
impl Rounding {
    pub const fn is_zero(&self) -> bool {
        self.top_right == 0.
            && self.top_right == 0.
            && self.bottom_left == 0.
            && self.bottom_right == 0.
    }
    pub const fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_left: value,
            bottom_right: value,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LineCap {
    Butt,
    #[default]
    Square,
    Round,
}
