use euclid::{
    default::{Point2D, Size2D},
    Box2D,
};
use graphics::{get_empty_mesh, Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseButton, MouseEvent, MouseEventKind};
use taffy::Layout;

use crate::{
    events::{
        BlurEvent, ChangeEvent, EventContext, EventHandler, EventPhase, FocusEvent, HandlesEvent,
        Redraw,
    },
    widgets::{Widget, WidgetInteractionState},
    Element,
};

#[derive(Clone)]
pub struct SliderWidget<M: Clone> {
    layout: Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
    pub focus_handler: EventHandler<FocusEvent, Self>,
    pub blur_handler: EventHandler<BlurEvent, Self>,
    pub change_handler: EventHandler<ChangeEvent<f32>, Self>,

    min_value: f32,
    max_value: f32,
    steps: u64,
    value: f32,

    state: WidgetInteractionState,
}

crate::macros::event_handlers::impl_event_handler! {
    SliderWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
    FocusEvent => focus_handler,
    BlurEvent => blur_handler,
    ChangeEvent<f32> => change_handler,
}

// impl<M: Clone> HandlesEvent<ChangeEvent<f32>> for SliderWidget<M> {
//     fn handler_mut(&mut self) -> &mut EventHandler<ChangeEvent<f32>, Self> {
//         &mut self.change_handler
//     }
// }

impl<M: Clone> SliderWidget<M> {
    pub fn new(steps: u64, initial_value: f32, min_value: f32, max_value: f32) -> Self {
        let state = WidgetInteractionState::new(false, false, false, true);
        Self {
            layout: Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
            focus_handler: EventHandler::none(),
            blur_handler: EventHandler::none(),
            change_handler: EventHandler::none(),

            min_value,
            max_value,
            steps,
            value: initial_value,

            state,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.state.active = active;
    }

    fn update_slider(&mut self, ctx: &mut EventContext<MouseEvent, M>) {
        let size = Size2D::new(self.layout.size.width, self.layout.size.height);
        let slider_area = Box2D::from_origin_and_size(
            Point2D::new(self.layout.location.x, self.layout.location.y),
            size,
        );

        let pos = ctx.payload().position;

        let percentage = ((pos.x - slider_area.min.x) / slider_area.width()).clamp(0., 1.);

        let range = self.max_value - self.min_value;
        let step_size = range / (self.steps) as f32;
        let step_index = (percentage * (self.steps) as f32).round();

        let value = self.min_value + (step_size * step_index);

        if self.value != value {
            ctx.request_redraw(Redraw::Now);
            ctx.push_event(crate::events::TreeEvent::ChangeEventF32(ChangeEvent {
                new: value,
                old: self.value,
            }));
            self.value = value;
        }
    }
}

impl<M: Clone> Element for SliderWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::Slider(self)
    }

    fn render(&mut self, _: &mut Systems, layout: Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            self.layout = layout;
        }
        get_empty_mesh()
    }

    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
        if ctx.current_phase() != EventPhase::Capturing {
            match ctx.payload().kind {
                MouseEventKind::Enter => self.state.hovered = true,
                MouseEventKind::Leave => {
                    self.state.pressed = false;
                    self.state.hovered = false;
                }
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    tracing::info!(
                        "slider requesting mouse capture for {:?}",
                        ctx.current_phase()
                    );
                    ctx.request_mouse_capture(ctx.current_node());
                    self.state.pressed = true;
                    self.update_slider(ctx);
                }
                MouseEventKind::Release { button, .. } if button == MouseButton::Left => {
                    if self.state.pressed {
                        ctx.request_mouse_release();
                        self.state.pressed = false;
                    }
                }
                MouseEventKind::Motion { .. } if self.state.pressed => {
                    self.update_slider(ctx);
                }
                _ => {}
            };
        }

        self.mouse_handler.clone().handle(self, ctx);
    }

    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, Self::Message>) {
        self.keyboard_handler.clone().handle(self, ctx);
    }
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent, Self::Message>) {
        self.focus_handler.clone().handle(self, ctx);
    }
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent, Self::Message>) {
        self.blur_handler.clone().handle(self, ctx);
    }
    fn change_event_f32(&mut self, ctx: &mut EventContext<ChangeEvent<f32>, Self::Message>) {
        self.change_handler.clone().handle(self, ctx);
    }

    fn is_dirty(&self) -> bool {
        false
    }

    fn clear_cache(&mut self) {
        self.layout = Layout::new();
    }

    fn focusable(&self) -> bool {
        true
    }
}
