use lyon::{
    geom::euclid::default::Transform3D,
    math::{Point, Size, Vector},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Projection {
    needs_rebinding: bool,
    /// Matrix taking world coords → viewport coords
    world_to_viewport: Transform3D<f32>,
    viewport: Size,
}

impl Projection {
    pub fn new(viewport: Size) -> Self {
        Self {
            needs_rebinding: true,
            world_to_viewport: Transform3D::identity(),
            viewport,
        }
    }

    pub fn needs_rebinding(&self) -> bool {
        self.needs_rebinding
    }
    pub fn mark_bound(&mut self) {
        self.needs_rebinding = false;
    }

    pub fn get_viewport(&self) -> Size {
        self.viewport
    }
    pub fn set_viewport(&mut self, new_size: Size) {
        self.viewport = new_size;
        self.world_to_viewport = Transform3D::identity();
        self.needs_rebinding = true;
    }

    /// Pan in world‐space by `delta` (in world units).
    pub fn pan_by(&mut self, delta: Vector) {
        let t = Transform3D::translation(delta.x, delta.y, 0.0);
        // world → viewport = (old_world_to_viewport) ∘ (translate_world)
        self.world_to_viewport = self.world_to_viewport.then(&t);
        self.needs_rebinding = true;
    }

    /// Zoom about a **viewport**‐space point `focus` by `factor`.
    /// `focus` is in viewport pixels.
    /// FIXME: doesn't properly zoom around focus
    pub fn zoom_at(&mut self, focus: Point, factor: f32) {
        // 1. move focus → origin in world coords
        let to_origin = Transform3D::translation(-focus.x, -focus.y, 0.0);
        // 2. scale
        let scale = Transform3D::scale(factor, factor, 1.0);
        // 3. move back
        let back = Transform3D::translation(focus.x, focus.y, 0.0);
        // zoom = back ∘ scale ∘ to_origin
        let zoom = to_origin.then(&scale).then(&back);

        // apply that *before* your existing world→viewport
        self.world_to_viewport = self.world_to_viewport.then(&zoom);
        self.needs_rebinding = true;
    }

    pub fn reset_zoom(&mut self) {
        self.world_to_viewport = Transform3D::identity();
        self.needs_rebinding = true;
    }

    /// Build a full world→UV (NDC) matrix: world → viewport → ortho → NDC
    pub fn world_to_uv(&self) -> Transform3D<f32> {
        let ortho = Transform3D::ortho(
            0.0,
            self.viewport.width,
            self.viewport.height,
            0.0,
            -1.0,
            1.0,
        );
        // world→uv = (world→viewport) ∘ ortho
        self.world_to_viewport.then(&ortho)
    }

    /// Just the viewport→UV step (no pan/zoom).
    pub fn viewport_to_uv(&self) -> Transform3D<f32> {
        Transform3D::ortho(
            0.0,
            self.viewport.width,
            self.viewport.height,
            0.0,
            -1.0,
            1.0,
        )
    }

    /// Map a point in viewport‐pixel space back to **world** coords.
    pub fn viewport_to_world(&self, p: Point) -> Point {
        // invert world→viewport, drop Z
        let inv = self.world_to_viewport.inverse().unwrap();
        // FIXME: returns None when zoomed out alot
        inv.transform_point3d(p.to_3d()).unwrap().to_2d()
    }
}
