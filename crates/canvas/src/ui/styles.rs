use euclid::default::Point2D;
use gui::prelude::*;

pub mod colors {
    use color::{AlphaColor, Srgb};

    pub const WHITE: AlphaColor<Srgb> = AlphaColor::WHITE;
    pub const BLACK: AlphaColor<Srgb> = AlphaColor::BLACK;
    pub const TRANSPARENT: AlphaColor<Srgb> = AlphaColor::TRANSPARENT;
    pub const RED: AlphaColor<Srgb> = AlphaColor::new([1., 0., 0., 1.]);
    pub const GREEN: AlphaColor<Srgb> = AlphaColor::new([0., 1., 0., 1.]);
    pub const BLUE: AlphaColor<Srgb> = AlphaColor::new([0., 0., 1., 1.]);

    pub const BACKGROUND: AlphaColor<Srgb> = AlphaColor::new([0.08, 0.08, 0.08, 1.]);
    pub const BORDER: AlphaColor<Srgb> = AlphaColor::new([0.9, 0.9, 0.9, 1.]);

    pub const BACKGROUND_ACCENT: AlphaColor<Srgb> = AlphaColor::new([0.12, 0.12, 0.12, 1.]);
    pub const BORDER_ACCENT: AlphaColor<Srgb> = AlphaColor::new([0.26, 0.26, 0.26, 1.]);
    pub const TEXT_ACCENT: AlphaColor<Srgb> = AlphaColor::new([0.65, 0.65, 0.65, 1.]);
    pub const BORDER_SELECTED: AlphaColor<Srgb> = AlphaColor::new([1., 1., 1., 1.]);

    pub const FOREGROUND: AlphaColor<Srgb> = AlphaColor::new([1., 1., 1., 1.]);
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
        grid_template_rows: vec![GridTemplateComponent::AUTO],
        grid_template_columns: vec![GridTemplateComponent::AUTO],
        ..Default::default()
    }
}
