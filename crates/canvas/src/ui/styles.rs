use euclid::default::Point2D;
use gui::prelude::*;

pub mod colors {
    use color::{PremulColor, Srgb};

    pub const WHITE: PremulColor<Srgb> = PremulColor::WHITE;
    pub const BLACK: PremulColor<Srgb> = PremulColor::BLACK;

    pub const BACKGROUND: PremulColor<Srgb> = PremulColor::new([0.08, 0.08, 0.08, 1.]);
    pub const BORDER: PremulColor<Srgb> = PremulColor::new([0.9, 0.9, 0.9, 1.]);

    pub const FOREGROUND: PremulColor<Srgb> = PremulColor::new([1., 1., 1., 1.]);
}

pub fn floating_grab(grab_area: f32, top_left: Point2D<f32>) -> Style {
    Style {
        display: Display::Grid,
        position: Position::Absolute,
        inset: Rect {
            left: LengthPercentageAuto::length(top_left.x),
            top: LengthPercentageAuto::length(top_left.y),
            right: LengthPercentageAuto::AUTO,
            bottom: LengthPercentageAuto::AUTO,
        },
        padding: Rect::<LengthPercentage>::length(grab_area),
        grid_template_rows: vec![TrackSizingFunction::AUTO],
        grid_template_columns: vec![TrackSizingFunction::AUTO],
        ..Default::default()
    }
}
