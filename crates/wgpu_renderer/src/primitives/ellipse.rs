use graphics_v2::primitives::Ellipse;
// use lyon::tessellation::{BuffersBuilder, FillVertex, VertexBuffers};

use crate::{Mesh, Vertex};

pub fn render_ellipse(_ellipse: &Ellipse) -> Mesh<Vertex> {
    // let mut buffers = VertexBuffers::<Vertex, u32>::new();
    // let mut builder = BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
    //     // Vertex::with_color(
    //     //     vertex.position(),
    //     //     C::apply(VertexKind::Color(ellipse.color)),
    //     // );
    //     todo!()
    // });
    todo!();

    // let options = FillOptions::tolerance(0.1);
    // let mut tessellator = FillTessellator::new();
    //
    // let tessellation_result = tessellator.tessellate_path(&self.path, &options, &mut builder);
    // if let Err(err) = tessellation_result {
    //     log::warn!(
    //         "Error while tessellating ellipse with options {:?}: {}",
    //         ellipse,
    //         err
    //     );
    // }

    // Mesh {
    //     vertices: buffers.vertices,
    //     indices: buffers.indices,
    // }
}
