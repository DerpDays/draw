use euclid::default::{Point2D, Vector2D};

pub mod primitives;
pub mod reactivity;

trait LayoutEqNoLocation {
    fn eq_no_location(&self, other: &Self) -> bool;
    fn location_delta(&self, old: &Self) -> Vector2D<f32>;
}

impl LayoutEqNoLocation for taffy::Layout {
    fn eq_no_location(&self, other: &Self) -> bool {
        taffy::Layout {
            location: taffy::Point::zero(),
            ..*self
        } == taffy::Layout {
            location: taffy::Point::zero(),
            ..*other
        }
    }

    fn location_delta(&self, old: &Self) -> Vector2D<f32> {
        Point2D::new(self.location.x, self.location.y)
            - Point2D::new(old.location.x, old.location.y)
    }
}
