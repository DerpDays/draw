use color::{AlphaColor, Srgb};
use euclid::default::Point2D;
use graphics::Rounding;
use gui_reactive::{
    prelude::*,
    reexports::reactive::{create_signal, Signal},
    tree::builder::ErasedBuilder,
    widgets::primitives::{div, DivOptions},
};

use crate::ui2::{drag_fn, floating_grab, styles::colors, DragState};

pub fn options() -> impl ErasedBuilder {
    let drag: Signal<Option<DragState>> = create_signal(None);
    let drag_style = create_signal(floating_grab(100., Point2D::new(500., 500.)));

    // root elemgeneric over
    div()
        .style(drag_style)
        .on_mouse(drag_fn(drag, drag_style))
        .child(
            div()
                .options(DivOptions {
                    bg_color: Some(colors::BACKGROUND.into()),
                    rounding: Some(Rounding::all(5.)),
                    stroke_width: Some(3.),
                    stroke_color: Some(AlphaColor::new([0.9, 0.9, 0.9, 1.]).into()),
                    ..Default::default()
                })
                .style(Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    padding: Rect::<LengthPercentage>::length(20.),
                    gap: Size::<LengthPercentage>::length(15.),
                    ..Style::DEFAULT
                }),
        )
}

trait OptionsDescriptor<'a> {
    fn descriptors(&'a self) -> impl Iterator<Item = DescriptorItem<'a>>;
}

pub enum DescriptorItem<'a> {
    Title(&'a str),
    Label(&'a str),
    Slider(),
    Radio(&'a [RadioItem]),
    ColorPicker(Signal<AlphaColor<Srgb>>),
}

pub struct RadioItem {}

pub fn create_options<'a>(options: &'a impl OptionsDescriptor<'a>) -> impl ErasedBuilder {
    let items = options
        .descriptors()
        .map(|descriptor| match descriptor {
            DescriptorItem::Title(_) => div(),
            DescriptorItem::Label(_) => div(),
            DescriptorItem::Slider() => div(),
            DescriptorItem::Radio(radio_items) => div(),
            DescriptorItem::ColorPicker(signal) => div(),
        })
        .collect::<Vec<_>>();
    div().child(items)
}

pub struct RectangleOptionsDescriptor {
    bg_color: Signal<AlphaColor<Srgb>>,
}

impl<'a> OptionsDescriptor<'a> for RectangleOptionsDescriptor {
    fn descriptors(&self) -> impl Iterator<Item = DescriptorItem<'a>> {
        [
            DescriptorItem::Title("Rectangle Tool"),
            DescriptorItem::Label("Background Color"),
            DescriptorItem::ColorPicker(self.bg_color),
        ]
        .into_iter()
    }
}
