use crate::scene::Scene;
use euclid::default::Point2D;
use graphics::{CanvasCoordinates, Primitive, Systems, Vertex};

use graphics::Mesh;
use renderer::GrowableMeshBuffer;
use wgpu::{Device, Queue};

pub struct Canvas {
    pub scene: Scene,

    pub render_cache: Mesh<Vertex>,
    pub scratch_render_cache: Option<Mesh<Vertex>>,

    pub scene_buffer: GrowableMeshBuffer,
    pub scratch_buffer: GrowableMeshBuffer,
}

impl Canvas {
    pub fn new(systems: &mut Systems) -> Self {
        let mut scene = Scene::default();

        let render_cache = scene.tessellate(systems);

        let scene_buffer = GrowableMeshBuffer::new(&systems.device, 1024, 2048);
        let scratch_buffer = GrowableMeshBuffer::new(&systems.device, 1024, 2048);

        Self {
            scene,

            render_cache,
            scratch_render_cache: None,

            scene_buffer,
            scratch_buffer,
        }
    }

    pub fn add_node(&mut self, systems: &mut Systems, node: Primitive<CanvasCoordinates>) {
        self.scene.add_node(node);
        self.render_cache = self.scene.tessellate(systems);
        _ = self.scene_buffer.replace_with_mesh(
            &systems.device,
            &systems.queue,
            &self.render_cache,
        );
    }

    pub fn remove_node_id(&mut self, systems: &mut Systems, id: u32) {
        self.scene.remove_node(id);
        self.render_cache = self.scene.tessellate(systems);
        _ = self.scene_buffer.replace_with_mesh(
            &systems.device,
            &systems.queue,
            &self.render_cache,
        );
    }
    pub fn get_node_at_position(
        &mut self,
        position: Point2D<f32>,
    ) -> Option<&mut Primitive<CanvasCoordinates>> {
        self.scene.get_node_at_position(position)
    }
    pub fn get_node_id_at_position(&mut self, position: Point2D<f32>) -> Option<u32> {
        self.scene.get_node_id_at_position(position)
    }

    pub fn clear_scratch(&mut self) {
        self.scratch_buffer.reset_no_wipe();
        self.scratch_render_cache = None;
    }
    pub fn update_scratch(&mut self, device: &Device, queue: &Queue, mesh: Mesh<Vertex>) {
        _ = self.scratch_buffer.replace_with_mesh(device, queue, &mesh);
        self.scratch_render_cache = Some(mesh);
    }
}
