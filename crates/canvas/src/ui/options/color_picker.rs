use std::marker::PhantomData;

use color::{AlphaColor, Hsl, Rgba8, Srgb};
use graphics::primitives::{RectangleOptions, TextOptions};
use graphics::{BasicLinearGradient, Rounding};

use gui::prelude::*;
use gui::tree::UITree;
use gui::widgets::{BackgroundWidget, SliderWidget, TextInputWidget, TextWidget, Widget};

use input::{CursorIcon, MouseEventKind, SpecialKey};

use crate::ui::options::OptionsMessage;
use crate::ui::styles::colors;
use crate::ui::Message;

#[derive(Clone, Copy, Debug)]
pub enum ColorPickerMessage {
    UpdateColor(AlphaColor<Srgb>),
    UpdateHue(f32),
    UpdateSaturation(f32),
    UpdateLightness(f32),

    UpdateHex([u8; 3]),
    UpdateOpacity(f32),
    ResetFields,
}

impl From<ColorPickerMessage> for Message {
    fn from(value: ColorPickerMessage) -> Self {
        OptionsMessage::ColorPicker(value).into()
    }
}

pub struct ColorPickerTree {
    pub container: NodeId,

    pub color: AlphaColor<Hsl>,

    hue_slider: SliderTree<6, AlphaColor<Hsl>>,
    saturation_slider: SliderTree<1, AlphaColor<Hsl>>,
    lightness_slider: SliderTree<2, AlphaColor<Hsl>>,
    hex_text_input: NodeId,
    opacity_text_input: NodeId,
}

fn expand_hex_string(hex: &str) -> Option<String> {
    let mut hex = hex.strip_prefix('#').unwrap_or(hex).to_string();

    if hex.len() != 3 && hex.len() != 6 {
        return None;
    }
    // Expand shorthand form (#RGB to #RRGGBB)
    if hex.len() == 3 {
        hex = hex
            .chars()
            .map(|c| format!("{0}{0}", c))
            .collect::<String>();
    }
    Some(hex)
}

fn parse_hex_string(hex: &str) -> Option<[u8; 3]> {
    let hex = expand_hex_string(hex)?;

    // Parse into u8 values
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some([r, g, b])
}

pub fn indicator_color_func(color: AlphaColor<Hsl>) -> AlphaColor<Srgb> {
    let lightness = color.components[2];
    if lightness < 75. {
        AlphaColor::WHITE
    } else {
        AlphaColor::BLACK
    }
}

pub fn color_picker_container_style() -> Style {
    Style {
        display: Display::None,
        flex_direction: FlexDirection::Column,
        gap: Size::from_length(10.),
        padding: Rect::length(10.),
        ..Default::default()
    }
}

impl ColorPickerTree {
    fn update_hex_field(&mut self, tree: &mut UITree<Widget<Message>>) {
        let Rgba8 { r, g, b, .. } = self.color.to_rgba8();
        tree.get_node_mut(self.hex_text_input)
            .as_text_input_mut()
            .expect("hex field is of type text input")
            .update_content_silent_no_redraw(format!("{:02X}{:02X}{:02X}", r, g, b));
    }

    fn update_opacity_field(&mut self, tree: &mut UITree<Widget<Message>>) {
        let alpha = ((self.color.components[3] * 100.) as u8).clamp(0, 100);
        tree.get_node_mut(self.opacity_text_input)
            .as_text_input_mut()
            .expect("hex field is of type text input")
            .update_content_silent_no_redraw(alpha.to_string());
    }

    pub fn update(&mut self, tree: &mut UITree<Widget<Message>>, message: &ColorPickerMessage) {
        match message {
            ColorPickerMessage::UpdateColor(color) => {
                self.color = color.convert();
                let [h, s, l, _] = self.color.components;
                self.hue_slider.update_value(tree, h * 10.);
                self.hue_slider.update_background(tree, self.color);
                self.saturation_slider.update_value(tree, s * 10.);
                self.saturation_slider.update_background(tree, self.color);
                self.lightness_slider.update_value(tree, l * 10.);
                self.lightness_slider.update_background(tree, self.color);
                self.update_hex_field(tree);
                self.update_opacity_field(tree);
            }
            ColorPickerMessage::UpdateHue(value) => {
                let [_, s, l, a] = self.color.convert::<Hsl>().components;
                self.color = AlphaColor::<Hsl>::new([*value, s, l, a]);
                self.hue_slider.update_indicator(tree, self.color);
                self.saturation_slider.update_background(tree, self.color);
                self.lightness_slider.update_background(tree, self.color);
                self.update_hex_field(tree);
            }
            ColorPickerMessage::UpdateSaturation(value) => {
                // FIXME: if lightness is min or max, the hue is not changed
                let [h, _, l, a] = self.color.convert::<Hsl>().components;
                self.color = AlphaColor::<Hsl>::new([h, *value, l, a]);
                self.hue_slider.update_background(tree, self.color);
                self.saturation_slider.update_indicator(tree, self.color);
                self.lightness_slider.update_background(tree, self.color);
                self.update_hex_field(tree);
            }
            ColorPickerMessage::UpdateLightness(value) => {
                let [h, s, _, a] = self.color.convert::<Hsl>().components;
                self.color = AlphaColor::<Hsl>::new([h, s, *value, a]);
                self.hue_slider.update_background(tree, self.color);
                self.saturation_slider.update_background(tree, self.color);
                self.lightness_slider.update_indicator(tree, self.color);
                self.update_hex_field(tree);
            }

            ColorPickerMessage::UpdateHex([r, g, b]) => {
                self.color = AlphaColor::from_rgb8(*r, *g, *b)
                    .with_alpha(self.color.components[3])
                    .convert();
                self.hue_slider.update_background(tree, self.color);
                self.saturation_slider.update_background(tree, self.color);
                self.lightness_slider.update_background(tree, self.color);
            }
            ColorPickerMessage::UpdateOpacity(opacity) => {
                tracing::info!("updating opacity to: {opacity:?}");
                self.color = self.color.with_alpha(*opacity);
                self.hue_slider.update_background(tree, self.color);
                self.saturation_slider.update_background(tree, self.color);
                self.lightness_slider.update_background(tree, self.color);
            }
            ColorPickerMessage::ResetFields => {
                self.update_hex_field(tree);
                self.update_opacity_field(tree);
            }
        }
    }
    pub fn build(tree: &mut UITree<Widget<Message>>) -> Self {
        let container = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: colors::BACKGROUND_ACCENT.into(),
                // stroke_color: colors::BORDER_ACCENT.into(),
                // stroke_width: 4.,
                rounding: Rounding::all(10.),
                box_sizing: graphics::BoxSizing::BorderBox,
                ..RectangleOptions::DEFAULT
            })
            .as_widget(),
            color_picker_container_style(),
        );

        let initial_color = AlphaColor::from_rgb8(255, 0, 0);
        let [hue, saturation, lightness, _] = initial_color.convert::<Hsl>().components;

        let hue_label = tree.new_leaf(
            TextWidget::new(
                "Hue".to_string(),
                TextOptions {
                    color: colors::TEXT_ACCENT.into(),
                    font_size: 14.,
                    ..TextOptions::default()
                },
            )
            .as_widget(),
            Style::DEFAULT,
        );
        tree.add_child(container, hue_label);
        let hue_slider = SliderTree::new(
            tree,
            container,
            (0., 3600.),
            3600,
            hue * 10.,
            |color: AlphaColor<Hsl>| {
                std::array::from_fn(|i| {
                    let start_hue = i as f32 * 60.;
                    let [_, s, l, a] = color.components;
                    RectangleOptions::only_color(BasicLinearGradient::new(
                        AlphaColor::<Hsl>::new([start_hue, s, l, a]).convert(),
                        AlphaColor::<Hsl>::new([start_hue + 60., s, l, a]).convert(),
                    ))
                })
            },
            initial_color.convert(),
            indicator_color_func,
            |val| ColorPickerMessage::UpdateHue(val / 10.).into(),
        );
        let saturation_label = tree.new_leaf(
            TextWidget::new(
                "Saturation".to_string(),
                TextOptions {
                    color: colors::TEXT_ACCENT.into(),
                    font_size: 14.,
                    ..TextOptions::default()
                },
            )
            .as_widget(),
            Style::DEFAULT,
        );
        tree.add_child(container, saturation_label);
        let saturation_slider = SliderTree::new(
            tree,
            container,
            (0., 1000.),
            1000,
            saturation * 10.,
            |color: AlphaColor<Hsl>| {
                std::array::from_fn(|_| {
                    let [h, _, l, a] = color.components;
                    RectangleOptions::only_color(BasicLinearGradient::new(
                        AlphaColor::<Hsl>::new([h, 0., l, a]).convert(),
                        AlphaColor::<Hsl>::new([h, 100., l, a]).convert(),
                    ))
                })
            },
            initial_color.convert(),
            indicator_color_func,
            |val| ColorPickerMessage::UpdateSaturation(val / 10.).into(),
        );

        let lightness_label = tree.new_leaf(
            TextWidget::new(
                "Lightness".to_string(),
                TextOptions {
                    color: colors::TEXT_ACCENT.into(),
                    font_size: 14.,
                    ..TextOptions::default()
                },
            )
            .as_widget(),
            Style::DEFAULT,
        );
        tree.add_child(container, lightness_label);
        let lightness_slider = SliderTree::new(
            tree,
            container,
            (0., 1000.),
            1000,
            lightness * 10.,
            |color: AlphaColor<Hsl>| {
                std::array::from_fn(|i| {
                    let [h, s, _, a] = color.components;
                    RectangleOptions::only_color(BasicLinearGradient::new(
                        AlphaColor::<Hsl>::new([h, s, (i * 50) as f32, a]).convert(),
                        AlphaColor::<Hsl>::new([h, s, ((i + 1) * 50) as f32, a]).convert(),
                    ))
                })
            },
            initial_color.convert(),
            indicator_color_func,
            |val| ColorPickerMessage::UpdateLightness(val / 10.).into(),
        );

        let text_container = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: colors::BACKGROUND_ACCENT.into(),
                stroke_color: colors::BORDER_ACCENT.into(),
                stroke_width: 2.,
                rounding: Rounding::all(4.),
                ..RectangleOptions::DEFAULT
            })
            .as_widget(),
            Style {
                display: Display::Flex,
                justify_content: Some(JustifyContent::SpaceBetween),
                align_items: Some(AlignItems::Center),
                size: Size {
                    width: Dimension::percent(1.),
                    height: Dimension::auto(),
                },
                padding: Rect::length(4.),
                ..Style::DEFAULT
            },
        );
        tree.add_child(container, text_container);

        let hex_container = tree.new_leaf(
            Widget::Layout,
            Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                gap: Size::length(5.),
                padding: Rect {
                    left: LengthPercentage::length(5.),
                    ..Rect::zero()
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(text_container, hex_container);

        let prefix = tree.new_leaf(
            TextWidget::new(
                "#".to_string(),
                TextOptions {
                    color: colors::TEXT_ACCENT.into(),
                    ..TextOptions::default()
                },
            )
            .as_widget(),
            Style::DEFAULT,
        );
        tree.add_child(hex_container, prefix);

        let hex_input_widget = tree.new_leaf(
            hex_input_widget(initial_color),
            Style {
                min_size: Size {
                    width: Dimension::length(100.),
                    height: Dimension::auto(),
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(hex_container, hex_input_widget);

        let opacity_container = tree.new_leaf(
            Widget::Layout,
            Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::Center),
                gap: Size::length(5.),
                padding: Rect {
                    right: LengthPercentage::length(5.),
                    ..Rect::zero()
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(text_container, opacity_container);

        let separator = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions::only_color(colors::BORDER_ACCENT)).as_widget(),
            Style {
                size: Size {
                    width: Dimension::length(2.),
                    height: Dimension::percent(1.),
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(opacity_container, separator);

        let opacity_input_widget = tree.new_leaf(
            opacity_input_widget(initial_color),
            Style {
                min_size: Size {
                    width: Dimension::length(20.),
                    height: Dimension::auto(),
                },
                ..Style::DEFAULT
            },
        );

        tree.add_child(opacity_container, opacity_input_widget);

        let suffix = tree.new_leaf(
            TextWidget::new(
                "%".to_string(),
                TextOptions {
                    color: colors::TEXT_ACCENT.into(),
                    ..TextOptions::default()
                },
            )
            .as_widget(),
            Style::DEFAULT,
        );
        tree.add_child(opacity_container, suffix);

        Self {
            color: AlphaColor::from_rgb8(255, 0, 0).convert(),

            container,
            hue_slider,
            saturation_slider,
            lightness_slider,
            hex_text_input: hex_input_widget,
            opacity_text_input: opacity_input_widget,
        }
    }
}

fn hex_input_widget(initial_color: AlphaColor<Srgb>) -> Widget<Message> {
    let Rgba8 { r, g, b, .. } = initial_color.to_rgba8();
    TextInputWidget::new(
        format!("{:02X}{:02X}{:02X}", r, g, b),
        None,
        TextOptions {
            color: colors::WHITE,
            ..TextOptions::default()
        },
    )
    .keyboard_handler(|this, ctx| {
        if ctx.in_capture_phase() {
            return;
        }
        ctx.prevent_default();
        match &ctx.payload().kind {
            input::KeyboardEventKind::Press(key) => match key {
                input::Key::SpecialKey(special_key) => match special_key {
                    SpecialKey::Escape => {
                        ctx.request_kb_focus_release();
                    }
                    SpecialKey::Delete => {
                        this.update_content("", ctx, false);
                        // Todo: push message
                    }
                    SpecialKey::Enter => {
                        ctx.request_kb_focus_release();
                    }
                    SpecialKey::Backspace => {
                        if this.content().len() > 0 {
                            let mut new_content = this.content().clone();
                            new_content.pop();
                            this.update_content(new_content, ctx, false);
                        }
                    }
                    SpecialKey::Home => todo!(),
                    SpecialKey::End => todo!(),
                    SpecialKey::Left => todo!(),
                    SpecialKey::Right => todo!(),
                    SpecialKey::Down => todo!(),
                    SpecialKey::Up => todo!(),
                    SpecialKey::Tab => todo!(),
                    _ => {}
                },
                input::Key::Character(str) => {
                    if let Some(char) = str.chars().next() {
                        if char.is_digit(16) && this.content().len() < 6 {
                            let mut new_content = this.content().clone();
                            new_content.push(char.to_ascii_uppercase());
                            this.update_content(new_content, ctx, false);
                        };
                    }
                }
                input::Key::Unknown => {}
            },
            input::KeyboardEventKind::ModifiersChanged => {}
            _ => {}
        };
    })
    .change_handler(|this, ctx| {
        if let Some(color) = parse_hex_string(this.content()) {
            ctx.push_messages(vec![ColorPickerMessage::UpdateHex(color).into()]);
        }
        ctx.request_redraw(Redraw::Now);
    })
    .focus_handler(|_, _| {
        tracing::info!("got focus event!");
    })
    .blur_handler(|this, ctx| {
        tracing::info!("got blur event!");
        if !ctx.in_capture_phase() {
            if let Some(expanded) = expand_hex_string(this.content()) {
                this.update_content(expanded, ctx, false);
            } else {
                ctx.push_messages(vec![ColorPickerMessage::ResetFields.into()]);
                ctx.request_redraw(Redraw::Now);
            }
        }
    })
    .as_widget()
}
fn opacity_input_widget(initial_color: AlphaColor<Srgb>) -> Widget<Message> {
    let Rgba8 { a, .. } = initial_color.to_rgba8();
    TextInputWidget::new(
        format!("{}", ((a as f32 / 255.) * 100.).round()),
        None,
        TextOptions {
            color: colors::WHITE,
            ..TextOptions::default()
        },
    )
    .keyboard_handler(|this, ctx| {
        if ctx.in_capture_phase() {
            return;
        }
        ctx.prevent_default();
        match &ctx.payload().kind {
            input::KeyboardEventKind::Press(key) => match key {
                input::Key::SpecialKey(special_key) => match special_key {
                    SpecialKey::Escape => {
                        ctx.request_kb_focus_release();
                    }
                    SpecialKey::Delete => {
                        this.update_content("", ctx, false);
                        // Todo: push message
                    }
                    SpecialKey::Enter => {
                        ctx.request_kb_focus_release();
                    }
                    SpecialKey::Backspace => {
                        if this.content().len() > 0 {
                            let mut new_content = this.content().clone();
                            new_content.pop();
                            this.update_content(new_content, ctx, false);
                        }
                    }
                    SpecialKey::Home => todo!(),
                    SpecialKey::End => todo!(),
                    SpecialKey::PageUp => todo!(),
                    SpecialKey::PageDown => todo!(),
                    SpecialKey::Left => todo!(),
                    SpecialKey::Right => todo!(),
                    SpecialKey::Up => {
                        let new_opacity = this
                            .content()
                            .parse::<u8>()
                            .map_or(1, |x| (x + 1).clamp(0, 100));
                        this.update_content(new_opacity.to_string(), ctx, false);
                    }
                    SpecialKey::Down => {
                        let new_opacity = this
                            .content()
                            .parse::<u8>()
                            .map_or(0, |x| (x - 1).clamp(0, 100));
                        this.update_content(new_opacity.to_string(), ctx, false);
                    }
                    SpecialKey::Tab => todo!(),
                    _ => {}
                },
                input::Key::Character(str) => {
                    if let Some(char) = str.chars().next() {
                        if char.is_digit(10) && this.content().len() < 3 {
                            let mut new_content = this.content().clone();
                            new_content.push(char.to_ascii_uppercase());
                            if new_content.len() == 3
                                && !new_content.get(0..0).unwrap().contains(['1', '0'])
                            {
                                new_content.replace_range(0..1, "1");
                            }
                            this.update_content(new_content, ctx, false);
                        };
                    }
                }
                input::Key::Unknown => {}
            },
            input::KeyboardEventKind::ModifiersChanged => {}
            _ => {}
        };
    })
    .change_handler(|this, ctx| {
        let opacity = (this.content().parse::<u8>().unwrap_or(0) as f32 / 100.).clamp(0., 1.);
        ctx.push_messages(vec![ColorPickerMessage::UpdateOpacity(opacity).into()]);
        ctx.request_redraw(Redraw::Now);
    })
    .focus_handler(|_, _| {
        tracing::info!("got focus event!");
    })
    .blur_handler(|this, ctx| {
        tracing::info!("got blur event!");
        if !ctx.in_capture_phase() {
            if this.content().is_empty() {
                this.update_content("100".to_string(), ctx, false);
            }
        }
    })
    .as_widget()
}

struct SliderTree<const N: usize, T> {
    slider: NodeId,
    indicator_container: NodeId,
    indicator_color: NodeId,
    background_parts: [NodeId; N],
    background_fn: fn(T) -> [RectangleOptions; N],
    indicator_color_fn: fn(T) -> AlphaColor<Srgb>,
    _marker: PhantomData<T>,
}

impl<const N: usize, T: Clone> SliderTree<N, T> {
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

    pub fn update_value(&self, tree: &mut UITree<Widget<Message>>, value: f32) {
        let slider = tree
            .get_node_mut(self.slider)
            .as_slider_mut()
            .expect("slider node is always a slider widget");
        slider.update_content_silent_no_redraw(value);
        let range = slider.range();
        // value clamped & snapped into the right steps.
        let value = slider.value();

        tree.set_style(
            self.indicator_container,
            Self::slider_indicator_style(value, range.0, range.1),
        );
    }
    pub fn update_background(&self, tree: &mut UITree<Widget<Message>>, value: T) {
        let individual_part_backgrounds = (self.background_fn)(value.clone());
        for (idx, node_id) in self.background_parts.iter().enumerate() {
            let widget = tree.get_node_mut(*node_id).as_background_mut().unwrap();
            widget.change_options(individual_part_backgrounds[idx]);
        }
        self.update_indicator(tree, value);
    }
    pub fn update_indicator(&self, tree: &mut UITree<Widget<Message>>, value: T) {
        let indicator_color = (self.indicator_color_fn)(value);
        tree.get_node_mut(self.indicator_color)
            .as_background_mut()
            .unwrap()
            .change_options(RectangleOptions::only_color(indicator_color));
    }

    pub fn new(
        tree: &mut UITree<Widget<Message>>,
        root: NodeId,
        range: (f32, f32),
        steps: u64,
        initial_value: f32,
        background_fn: fn(T) -> [RectangleOptions; N],
        background_init_val: T,
        indicator_color_fn: fn(T) -> AlphaColor<Srgb>,
        on_change: impl Fn(f32) -> Message + 'static,
    ) -> SliderTree<N, T> {
        let indicator_container = tree.new_leaf(
            Widget::Layout,
            Self::slider_indicator_style(initial_value, range.0, range.1),
        );
        let indicator_color = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions::only_color(indicator_color_fn(
                background_init_val.clone(),
            )))
            .as_widget(),
            Style {
                position: Position::Relative,
                size: Size {
                    width: Dimension::length(3.),
                    height: Dimension::percent(1.),
                },
                inset: Rect {
                    left: LengthPercentageAuto::length(-1.5),
                    ..Rect::zero()
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(indicator_container, indicator_color);

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
                        node: indicator_container.clone(),
                        style: Self::slider_indicator_style(val, range.0, range.1),
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
            indicator_container,
            indicator_color,
            background_parts,
            background_fn: background_fn,
            indicator_color_fn,
            _marker: PhantomData,
        }
    }
}
