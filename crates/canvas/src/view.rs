use euclid::default::Size2D;
use graphics::{
    systems::{SystemsOwned, TextState, TextureState},
    Drawable,
};
use tracing::{info, instrument};

use gui::prelude::{EventResult, Redraw};
use gui::UITree;
use input::{CursorIcon, KeyboardEvent, Modifiers, MouseEvent};
use renderer::GrowableMeshBuffer;

use crate::{
    canvas::Canvas,
    pipeline::{Binds, DrawPipeline, ProjectionBind},
    projection::Projection,
    tools::{ToolKind, ToolMessage, Tools},
    ui::{Application, Message},
    RedrawRequest,
};

use crate::ui::options::OptionsTree;

pub struct View<T: RedrawRequest + Clone + 'static> {
    pub canvas: Canvas,
    pub app: Application,

    pub systems: SystemsOwned,
    pub projection: Projection,

    pub last_interaction: InteractionKind,
    pub focused_tool: Option<ToolKind>,
    pub tools: Tools,
    pub binds: Option<Binds>,

    pub redraw_manager: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionKind {
    Tool(ToolKind),
    Gui,
}

impl<T: RedrawRequest + Clone + 'static> View<T> {
    pub fn new(
        renderer: &renderer::State,
        viewport: Size2D<f32>,
        scale_factor: f64,
        redraw_manager: T,
    ) -> Self {
        let text = TextState::default();
        let texture = TextureState::new(&renderer.device);
        let projection = Projection::new(viewport);
        let mut systems = SystemsOwned::new(text, texture);

        let canvas = Canvas::new(&mut systems.to_ref(&renderer.device, &renderer.queue));

        let mut gui_buffer = GrowableMeshBuffer::new(&renderer.device, 1024, 2048);
        let mut gui = UITree::new(viewport, scale_factor);

        let root_node = gui.root_node();

        let (toolbar_id, tool_nodes) =
            crate::ui::toolbar::create_toolbar(&mut gui, ToolKind::default());
        _ = gui.add_child(gui.root_node(), toolbar_id);

        let options = OptionsTree::build(&mut gui, root_node);

        _ = gui_buffer.replace_with_mesh(
            &renderer.device,
            &renderer.queue,
            gui.render(&mut systems.to_ref(&renderer.device, &renderer.queue)),
        );

        let app = Application {
            gui,
            gui_buffer,

            drag_start: None,

            selected_tool: ToolKind::default(),
            tool_nodes,

            options,

            modifiers: Modifiers::empty(),
        };

        Self {
            canvas,

            app,
            systems,
            projection,

            last_interaction: InteractionKind::Gui,
            focused_tool: None,
            tools: Tools::default(),

            binds: None,
            redraw_manager,
        }
    }

    pub fn update_viewport(&mut self, viewport: Size2D<f32>, scale_factor: f64) {
        self.app.gui.update_viewport(viewport, scale_factor);
        self.projection.set_viewport(viewport);
        self.redraw_manager.request_redraw();
        info!("Updated the viewport size to: {viewport:?}!");
    }

    pub fn keyboard_event(&mut self, event: KeyboardEvent, renderer: &renderer::State) {
        self.app.modifiers = event.modifiers;

        // If we are currently focused on a tool, pass the event to the tool handler .
        if let Some(tool) = &self.focused_tool {
            let messages = tool.keyboard_event(
                &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                &mut self.tools,
                event,
            );
            self.handle_tool(*tool, messages, renderer);
            return;
        };
        // Otherwise pass the event to the gui event handler
        if let Some(events) = self.app.gui.keyboard_event(event.clone()) {
            self.handle_gui(events);
        } else {
            let messages = self.app.selected_tool.keyboard_event(
                &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                &mut self.tools,
                event,
            );
            self.handle_tool(self.app.selected_tool, messages, renderer);
        }
    }

    // TODO: support multiple pointers.
    pub fn mouse_event(
        &mut self,
        event: MouseEvent,
        renderer: &renderer::State,
    ) -> Option<CursorIcon> {
        // If we are currently focused on a tool, handle the event for the tool.
        if let Some(tool) = self.focused_tool {
            self.last_interaction = InteractionKind::Tool(tool);
            let messages = tool.mouse_event(
                &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                &mut self.tools,
                event,
                self.app.modifiers,
                &self.projection,
            );
            return self.handle_tool(tool, messages, renderer);
        }

        // Otherwise, pass the event to the gui, which returns none if it did not hit.
        if let Some(result) = self.app.gui.mouse_event(event.clone()) {
            if self.last_interaction != InteractionKind::Gui {
                let messages = self.app.selected_tool.mouse_event(
                    &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                    &mut self.tools,
                    MouseEvent::leave(event.position),
                    self.app.modifiers,
                    &self.projection,
                );
                self.handle_tool(self.app.selected_tool, messages, renderer);
            };

            self.last_interaction = InteractionKind::Gui;
            return self.handle_gui(result);
        }
        // Otherwise, since the event wasn't for the gui, pass it on to the selected tool,
        // here we need to handle enter/exit events for the tools if the selected tool has changed.
        let mut result = None;
        match self.last_interaction {
            InteractionKind::Tool(tool) if self.app.selected_tool != tool => {
                let leave_messages = tool.mouse_event(
                    &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                    &mut self.tools,
                    MouseEvent::leave(event.position),
                    self.app.modifiers,
                    &self.projection,
                );
                self.handle_tool(tool, leave_messages, renderer);
                let enter_messages = self.app.selected_tool.mouse_event(
                    &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                    &mut self.tools,
                    MouseEvent::enter(event.position),
                    self.app.modifiers,
                    &self.projection,
                );
                result = self
                    .handle_tool(self.app.selected_tool, enter_messages, renderer)
                    .or(Some(self.app.selected_tool.default_cursor()))
            }
            InteractionKind::Gui => {
                let messages = self.app.selected_tool.mouse_event(
                    &mut self.systems.to_ref(&renderer.device, &renderer.queue),
                    &mut self.tools,
                    MouseEvent::enter(event.position),
                    self.app.modifiers,
                    &self.projection,
                );
                result = self
                    .handle_tool(self.app.selected_tool, messages, renderer)
                    .or(Some(self.app.selected_tool.default_cursor()))
            }
            _ => {}
        }
        self.last_interaction = InteractionKind::Tool(self.app.selected_tool);

        let messages = self.app.selected_tool.mouse_event(
            &mut self.systems.to_ref(&renderer.device, &renderer.queue),
            &mut self.tools,
            event,
            self.app.modifiers,
            &self.projection,
        );
        self.handle_tool(self.app.selected_tool, messages, renderer)
            .or(result)
    }

    pub fn handle_gui(&mut self, events: EventResult<Message>) -> Option<CursorIcon> {
        let mut cursor_icon = None;
        for (node, messages) in events.messages() {
            for message in messages {
                cursor_icon =
                    crate::ui::handle_message(&mut self.app, *node, message, &self.redraw_manager)
                        .or(cursor_icon);
            }
        }

        if let Some(redraw) = events.is_requesting_redraw() {
            match redraw {
                Redraw::Now => self.redraw_manager.request_redraw(),
                Redraw::Duration(duration) => self.redraw_manager.request_redraw_duration(duration),
            }
        }
        cursor_icon
    }

    pub fn handle_tool(
        &mut self,
        tool: ToolKind,
        messages: Vec<ToolMessage>,
        renderer: &renderer::State,
    ) -> Option<CursorIcon> {
        let systems = &mut self.systems.to_ref(&renderer.device, &renderer.queue);
        let mut cursor_icon = None;
        for message in messages {
            match message {
                ToolMessage::CursorIcon(icon) => {
                    cursor_icon = Some(icon);
                }
                ToolMessage::Commit(primitive) => {
                    self.canvas.clear_scratch();
                    self.canvas.add_node(systems, primitive);
                    self.redraw_manager.request_redraw();
                }
                ToolMessage::Scratch(mesh) => {
                    self.canvas
                        .update_scratch(&systems.device, &systems.queue, mesh);
                    self.redraw_manager.request_redraw();
                }
                ToolMessage::ClearScratch => {
                    self.canvas.clear_scratch();
                    self.redraw_manager.request_redraw();
                }
                ToolMessage::ChangePrimaryColor(_color) => {
                    tracing::info!("primary color changed aaaa")
                }
                ToolMessage::SetFocus => self.focused_tool = Some(tool),
                ToolMessage::ReleaseFocus => {
                    self.focused_tool = None;
                }
                ToolMessage::Select(point) => {
                    cursor_icon = Some(CursorIcon::Grabbing);
                    if let Some(node) = self.canvas.get_node_at_position(point) {
                        tracing::info!("attempted to select a node : {node:?}");
                    };
                }
                ToolMessage::GrabMove(origin, position) => {
                    self.projection.pan_by(position - origin);
                    self.redraw_manager.request_redraw();
                }
                ToolMessage::Erase(point) => {
                    if let Some(id) = self.canvas.get_node_id_at_position(point) {
                        self.canvas.remove_node_id(systems, id);
                        self.redraw_manager.request_redraw();
                    };
                }
                ToolMessage::ZoomIn(point) => {
                    self.projection.zoom_at(point, 1.1);
                    self.redraw_manager.request_redraw();
                }
                ToolMessage::ZoomOut(point) => {
                    self.projection.zoom_at(point, 0.9);
                    self.redraw_manager.request_redraw();
                }
                ToolMessage::ResetZoom => {
                    self.projection.reset_zoom();
                    self.redraw_manager.request_redraw();
                }
            };
        }
        cursor_icon
    }

    #[instrument(skip_all)]
    pub fn render(
        &mut self,
        state: &renderer::State,
        surface: &wgpu::Surface,
        pipeline: &DrawPipeline,
    ) {
        let start = std::time::Instant::now();
        let binds = self.binds.get_or_insert(Binds {
            projection: ProjectionBind::new(state, &pipeline.bind_group_layouts, &self.projection),
            texture_atlases: pipeline.bind_group_layouts.new_texture_atlas_bind_group(
                &state.device,
                &self.systems.texture.mask_atlas.texture_view,
                &self.systems.texture.color_atlas.texture_view,
                &pipeline.sampler,
            ),
        });
        if self.projection.needs_rebinding() {
            binds.projection.update_viewport(state, &self.projection);
            self.projection.mark_bound();
            info!("Updated the projection bind!");
        }
        if self.systems.texture.needs_rebinding() {
            // TODO: move this
            binds.texture_atlases = pipeline.bind_group_layouts.new_texture_atlas_bind_group(
                &state.device,
                &self.systems.texture.mask_atlas.texture_view,
                &self.systems.texture.color_atlas.texture_view,
                &pipeline.sampler,
            );
            self.systems.texture.mark_bound();
            info!("Updated the texture bind!");
        }
        if self.app.gui.is_dirty() {
            tracing::info!("gui is dirty, redrawing!");
            _ = self.app.gui_buffer.replace_with_mesh(
                &state.device,
                &state.queue,
                self.app
                    .gui
                    .render(&mut self.systems.to_ref(&state.device, &state.queue)),
            );
        }

        let frame = surface.get_current_texture().unwrap();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Canvas Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Canvas Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&pipeline.render_pipeline);
            render_pass.set_bind_group(0, &binds.projection.bind_group, &[]);
            render_pass.set_bind_group(1, &binds.texture_atlases, &[]);

            if self.canvas.scene_buffer.num_indices > 0 {
                render_pass.set_vertex_buffer(0, self.canvas.scene_buffer.vertex.buf.slice(..));
                render_pass.set_index_buffer(
                    self.canvas.scene_buffer.index.buf.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..self.canvas.scene_buffer.num_indices, 0, 0..1);
            }

            if self.canvas.scratch_buffer.num_indices > 0 {
                render_pass.set_vertex_buffer(0, self.canvas.scratch_buffer.vertex.buf.slice(..));
                render_pass.set_index_buffer(
                    self.canvas.scratch_buffer.index.buf.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..self.canvas.scratch_buffer.num_indices, 0, 0..1);
            }

            if self.app.gui_buffer.num_indices > 0 {
                tracing::trace!("drawing gui buffer new");
                render_pass.set_vertex_buffer(0, self.app.gui_buffer.vertex.buf.slice(..));
                render_pass.set_index_buffer(
                    self.app.gui_buffer.index.buf.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..self.app.gui_buffer.num_indices, 0, 0..1);
            }
        }

        state.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        tracing::trace!("Time taken to render: {:?}", start.elapsed());
    }
}
