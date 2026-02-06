use graphics::Primitive;
use input::{
    Key,
    KeyboardEvent,
    KeyboardEventKind,
    MouseButton,
    MouseEvent,
    MouseEventKind,
    SpecialKey,
};
use sycamore_reactive::{ReadSignal, Signal};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    MeasureCtx,
    prelude::{BlurEvent, EventContext},
    tree::{Widget, builder::ElementBuilder},
};

pub struct InputField<I, B>
where
    I: Fn(Signal<String>, KeyboardEventKind),
    B: Fn(Signal<String>),
{
    value: Signal<String>,

    enabled: ReadSignal<bool>,

    input_fn: I,
    blur_fn: B,
}

impl<I, B> Widget for InputField<I, B>
where
    I: Fn(Signal<String>, KeyboardEventKind),
    B: Fn(Signal<String>),
{
    fn render(&mut self, _: &Layout, _: &Style) -> Option<Primitive> {
        None
    }
    fn measure(
        &mut self,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }

    fn debug_label(&self) -> &'static str {
        "InputField"
    }

    fn focusable(&self) -> bool {
        true
    }

    fn default_mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>, _: &Layout) {
        if !ctx.in_capture_phase() {
            if self.enabled.is_alive() && !self.enabled.get_untracked() {
                return;
            }
            if let MouseEventKind::Press {
                button: MouseButton::Left,
                ..
            } = ctx.payload().kind
            {
                ctx.request_kb_focus_capture(ctx.current_node());
            }
        }
    }
    fn default_keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>, _: &Layout) {
        if !ctx.in_capture_phase() {
            if self.enabled.is_alive() && !self.enabled.get_untracked() {
                return;
            }
            (self.input_fn)(self.value, ctx.payload().kind.clone());
            if let KeyboardEventKind::Press(Key::SpecialKey(SpecialKey::Escape)) =
                ctx.payload().kind
            {
                ctx.request_kb_focus_release();
            }
        }
    }
    fn default_blur_event(&mut self, ctx: &mut EventContext<BlurEvent>, _: &Layout) {
        if !ctx.in_capture_phase() {
            (self.blur_fn)(self.value);
        }
    }
}

#[inline(always)]
pub fn input_field<I, B>(
    value: Signal<String>,
    enabled: ReadSignal<bool>,
    input_fn: I,
    blur_fn: B,
) -> ElementBuilder<InputField<I, B>>
where
    I: Fn(Signal<String>, KeyboardEventKind),
    B: Fn(Signal<String>),
{
    ElementBuilder::new(InputField {
        value,
        enabled,
        input_fn,
        blur_fn,
    })
}

#[inline(always)]
#[allow(clippy::type_complexity)]
pub fn color_hex_input_field(
    value: Signal<String>,
    enabled: ReadSignal<bool>,
) -> ElementBuilder<InputField<impl Fn(Signal<String>, KeyboardEventKind), impl Fn(Signal<String>)>>
{
    input_field(
        value,
        enabled,
        |signal, event| {
            let mut value = signal.get_clone_untracked();
            let mut updated = false;
            if let KeyboardEventKind::Press(key) = event {
                match key {
                    Key::SpecialKey(SpecialKey::Backspace) => {
                        value.pop();
                        updated = true;
                    }
                    Key::Character(string) => {
                        for i in string.chars().filter(|x| x.is_ascii_hexdigit()) {
                            if value.len() < 6 {
                                value.push(i.to_ascii_uppercase());
                            }
                        }
                        updated = true;
                    }
                    _ => {}
                }
                if updated {
                    signal.set(value)
                }
            }
        },
        |signal| {
            log::error!("got blur event!");
            let val = signal.get_clone_untracked();
            if val.len() != 6 {
                if val.len() == 3 {
                    signal.set(
                        val.chars()
                            .map(|c| format!("{0}{0}", c))
                            .collect::<String>(),
                    );
                } else {
                    signal.set(format!("{:0<6}", val).to_string())
                }
            };
        },
    )
}
