use std::marker::PhantomData;

use color::{Hsl, PremulColor};
use euclid::default::Point2D;
use graphics::primitives::{RectangleOptions, TextOptions};
use graphics::{BasicLinearGradient, Rounding};

use gui::prelude::*;
use gui::tree::{UITree, ZIndexProperties};
use gui::widgets::{BackgroundWidget, ContainerWidget, SliderWidget, TextWidget, Widget};

use input::{CursorIcon, MouseButton, MouseEvent, MouseEventKind};

use crate::ui::options::color_swatches::ColorSwatches;
use crate::ui::styles::{colors, floating_grab};
use crate::ui::Message;

mod color_swatches;
mod rectangle;

#[derive(Clone, Copy, Debug)]
pub enum OptionsMessage {
    ColorPicker(ColorPickerMessage),
}

impl From<OptionsMessage> for Message {
    fn from(value: OptionsMessage) -> Self {
        Message::ToolOptions(value)
    }
}

pub struct OptionsTree {
    pub grab_area_node: NodeId,
    pub background_node: NodeId,
    pub color_picker: ColorPickerTree,
}

pub fn grab_fn(_: &mut ContainerWidget<Message>, ctx: &mut EventContext<MouseEvent, Message>) {
    match ctx.current_phase() {
        EventPhase::Bubbling | EventPhase::AtTarget | EventPhase::Direct => {
            match ctx.payload().kind {
                MouseEventKind::Enter => {
                    ctx.push_messages(vec![Message::CursorIcon(CursorIcon::Grab)])
                }
                MouseEventKind::Leave => ctx.push_messages(vec![
                    Message::CursorIcon(CursorIcon::default()),
                    Message::EndGrab,
                ]),
                MouseEventKind::Motion { .. } => {
                    ctx.push_messages(vec![Message::HandleGrabMove(ctx.payload().position)])
                }
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    ctx.push_messages(vec![
                        Message::MoveTop(ctx.current_node()),
                        Message::CursorIcon(CursorIcon::Grabbing),
                        Message::StartGrab(ctx.payload().position),
                    ]);
                    ctx.request_mouse_capture(ctx.current_node());
                }
                MouseEventKind::Release { button, .. } if button == MouseButton::Left => {
                    ctx.push_messages(vec![
                        Message::CursorIcon(CursorIcon::Grab),
                        Message::EndGrab,
                    ]);
                    ctx.request_mouse_release();
                }
                _ => {}
            }
        }
        EventPhase::Capturing => match ctx.payload().kind {
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                ctx.push_messages(vec![Message::MoveTop(ctx.current_node())])
            }
            _ => {}
        },
    };
}

impl OptionsTree {
    pub fn update(&mut self, tree: &mut UITree<Widget<Message>>, message: &OptionsMessage) {
        match message {
            OptionsMessage::ColorPicker(message) => self.color_picker.update(tree, message),
        }
    }
    pub fn build(tree: &mut UITree<Widget<Message>>, root: NodeId) -> Self {
        let grab_area = ContainerWidget::new(true).mouse_handler(grab_fn);
        let grab_area_node = tree.new_leaf_with_z(
            grab_area.as_widget(),
            floating_grab(100., Point2D::new(100., 500.)),
            ZIndexProperties {
                z_index: 1,
                isolate_z: true,
            },
        );
        tree.add_child(root, grab_area_node);

        let background_node = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: colors::BACKGROUND.into(),
                rounding: Rounding::all(5.),
                stroke_width: 3.,
                stroke_color: PremulColor::new([0.9, 0.9, 0.9, 1.]).into(),
                box_sizing: graphics::BoxSizing::ContentBox,
            })
            .as_widget(),
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: Rect::<LengthPercentage>::length(20.),
                gap: Size::<LengthPercentage>::length(15.),
                ..Style::DEFAULT
            },
        );
        tree.add_child(grab_area_node, background_node);

        let color_picker_label = tree.new_leaf(
            TextWidget::new(
                "Color Picker".to_string(),
                TextOptions {
                    color: PremulColor::new([1., 1., 1., 1.]),
                    font_size: 16.,
                    ..Default::default()
                },
            )
            .as_widget(),
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: Rect {
                    // bottom: LengthPercentage::length(10.),
                    ..Rect::zero()
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(background_node, color_picker_label);
        let color_picker = ColorPickerTree::build(tree);

        let color_picker_container = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: colors::BACKGROUND.into(),
                rounding: Rounding::all(5.),
                stroke_width: 3.,
                stroke_color: PremulColor::new([0.9, 0.9, 0.9, 1.]).into(),
                box_sizing: graphics::BoxSizing::ContentBox,
            })
            .as_widget(),
            Style {
                position: Position::Absolute,
                inset: Rect {
                    left: LengthPercentageAuto::percent(1.25),
                    ..auto()
                },
                padding: Rect::length(10.),
                ..Style::DEFAULT
            },
        );
        tree.add_child(background_node, color_picker_container);
        tree.add_child(color_picker_container, color_picker.container);

        let swatches = ColorSwatches::build(
            tree,
            &[
                PremulColor::WHITE,
                PremulColor::BLACK,
                PremulColor::new([1., 0., 0., 1.]),
                PremulColor::new([0., 1., 0., 1.]),
            ],
        );
        tree.add_child(background_node, swatches.container);

        Self {
            grab_area_node,
            background_node,
            color_picker,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ColorPickerMessage {
    UpdateHue(f32),
    UpdateSaturation(f32),
    UpdateLightness(f32),
}

impl From<ColorPickerMessage> for Message {
    fn from(value: ColorPickerMessage) -> Self {
        OptionsMessage::ColorPicker(value).into()
    }
}

struct ColorPickerTree {
    container: NodeId,

    hue: f32,
    saturation: f32,
    lightness: f32,

    hue_slider: SliderTree<6, [f32; 3]>,
    saturation_slider: SliderTree<1, [f32; 3]>,
    lightness_slider: SliderTree<2, [f32; 3]>,
}

impl ColorPickerTree {
    pub fn update(&mut self, tree: &mut UITree<Widget<Message>>, message: &ColorPickerMessage) {
        match message {
            ColorPickerMessage::UpdateHue(value) => {
                self.hue = *value;
                self.saturation_slider
                    .update_background(tree, [self.hue, self.saturation, self.lightness]);
                self.lightness_slider
                    .update_background(tree, [self.hue, self.saturation, self.lightness]);
            }
            ColorPickerMessage::UpdateSaturation(value) => {
                self.saturation = *value;
                self.hue_slider
                    .update_background(tree, [self.hue, self.saturation, self.lightness]);
                self.lightness_slider
                    .update_background(tree, [self.hue, self.saturation, self.lightness]);
            }
            ColorPickerMessage::UpdateLightness(value) => {
                self.lightness = *value;
                self.hue_slider
                    .update_background(tree, [self.hue, self.saturation, self.lightness]);
                self.saturation_slider
                    .update_background(tree, [self.hue, self.saturation, self.lightness]);
            }
        }
    }
    pub fn build(tree: &mut UITree<Widget<Message>>) -> Self {
        let container = tree.new_leaf(
            Widget::Layout,
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                gap: Size::from_length(10.),
                ..Default::default()
            },
        );

        let initial_hsl = [0., 100., 50.];

        let hue_slider = SliderTree::new(
            tree,
            container,
            (0., 3600.),
            3600,
            initial_hsl[0] * 10.,
            |[_, saturation, lightness]| {
                std::array::from_fn(|i| {
                    let start_hue = i as f32 * 60.;
                    RectangleOptions::only_color(BasicLinearGradient::new(
                        PremulColor::<Hsl>::new([start_hue, saturation, lightness, 1.]).convert(),
                        PremulColor::<Hsl>::new([start_hue + 60., saturation, lightness, 1.])
                            .convert(),
                    ))
                })
            },
            initial_hsl,
            |val| ColorPickerMessage::UpdateHue(val / 10.).into(),
        );
        let saturation_slider = SliderTree::new(
            tree,
            container,
            (0., 1000.),
            1000,
            initial_hsl[1] * 10.,
            |[hue, _, lightness]| {
                std::array::from_fn(|i| {
                    RectangleOptions::only_color(BasicLinearGradient::new(
                        PremulColor::<Hsl>::new([hue, 0., lightness, 1.]).convert(),
                        PremulColor::<Hsl>::new([hue, 100., lightness, 1.]).convert(),
                    ))
                })
            },
            initial_hsl,
            |val| ColorPickerMessage::UpdateSaturation(val / 10.).into(),
        );
        let lightness_slider = SliderTree::new(
            tree,
            container,
            (0., 1000.),
            1000,
            initial_hsl[2] * 10.,
            |[hue, saturation, _]| {
                std::array::from_fn(|i| {
                    RectangleOptions::only_color(BasicLinearGradient::new(
                        PremulColor::<Hsl>::new([hue, saturation, (i * 50) as f32, 1.]).convert(),
                        PremulColor::<Hsl>::new([hue, saturation, ((i + 1) * 50) as f32, 1.])
                            .convert(),
                    ))
                })
            },
            initial_hsl,
            |val| ColorPickerMessage::UpdateLightness(val / 10.).into(),
        );
        Self {
            hue: initial_hsl[0],
            saturation: initial_hsl[1],
            lightness: initial_hsl[2],

            container,
            hue_slider,
            saturation_slider,
            lightness_slider,
        }
    }
}
fn slider_indicator_style(value: f32, start: f32, end: f32) -> Style {
    Style {
        position: Position::Absolute,
        size: Size::percent(1.),
        inset: Rect {
            left: LengthPercentageAuto::percent((value - start) / (end - start)),
            ..Rect::auto()
        },
        ..Style::DEFAULT
    }
}

struct SliderTree<const N: usize, T> {
    slider: NodeId,
    indicator: NodeId,
    background_parts: [NodeId; N],
    background_fn: fn(T) -> [RectangleOptions; N],
    _marker: PhantomData<T>,
}

impl<const N: usize, T> SliderTree<N, T> {
    pub fn update_background(&self, tree: &mut UITree<Widget<Message>>, value: T) {
        let individual_part_backgrounds = (self.background_fn)(value);
        for (idx, node_id) in self.background_parts.iter().enumerate() {
            let widget = tree.get_node_mut(*node_id).as_background_mut().unwrap();
            widget.change_options(individual_part_backgrounds[idx]);
        }
    }
    pub fn new(
        tree: &mut UITree<Widget<Message>>,
        root: NodeId,
        range: (f32, f32),
        steps: u64,
        initial_value: f32,
        background_fn: fn(T) -> [RectangleOptions; N],
        background_init_val: T,
        on_change: impl Fn(f32) -> Message + 'static,
    ) -> SliderTree<N, T> {
        let indicator_container = tree.new_leaf(
            Widget::Layout,
            slider_indicator_style(initial_value, range.0, range.1),
        );
        let indicator_node = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions::only_color(PremulColor::new([
                1., 1., 1., 1.,
            ])))
            .as_widget(),
            Style {
                position: Position::Relative,
                size: Size {
                    width: Dimension::length(3.),
                    height: Dimension::percent(1.),
                },
                inset: Rect {
                    left: LengthPercentageAuto::length(-1.5),
                    ..Rect::auto()
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(indicator_container, indicator_node);

        let indicator_container_clone = indicator_container;
        let slider_widget = SliderWidget::new(steps, initial_value, range.0, range.1)
            .mouse_handler(|_, ctx| {
                if ctx.current_phase() != EventPhase::Capturing {
                    if ctx.payload().kind == MouseEventKind::Enter {
                        ctx.push_messages(vec![Message::CursorIcon(CursorIcon::Pointer)]);
                    }
                    ctx.stop_propagation();
                }
            })
            .change_handler(move |_, ctx| {
                if ctx.current_phase() != EventPhase::Capturing {
                    let val = ctx.payload().new;
                    ctx.push_tree_command(gui::tree::TreeCommand::SetStyle {
                        node: indicator_container_clone,
                        style: slider_indicator_style(val, range.0, range.1),
                    });
                    ctx.push_messages(vec![on_change(val)]);
                }
            });

        let slider_style = Style {
            display: Display::Flex,
            size: Size::from_lengths(256., 36.),
            min_size: Size::from_lengths(192., 24.),
            ..Default::default()
        };
        let slider_node = tree.new_leaf(slider_widget.as_widget(), slider_style);
        tree.add_child(root, slider_node);

        // Build background parts colors
        let individual_part_backgrounds = background_fn(background_init_val);

        let background_parts = std::array::from_fn(|i| {
            let background_part = BackgroundWidget::new(individual_part_backgrounds[i]);
            let part_node = tree.new_leaf(
                background_part.as_widget(),
                Style {
                    flex_grow: 1.,
                    ..Default::default()
                },
            );
            tree.add_child(slider_node, part_node);
            part_node
        });

        tree.add_child(slider_node, indicator_container);

        SliderTree {
            slider: slider_node,
            indicator: indicator_container,
            background_parts,
            background_fn: background_fn,
            _marker: PhantomData,
        }
    }
}
