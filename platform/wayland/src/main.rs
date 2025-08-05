mod logging;

use anyhow::{bail, Context, Result};
use tracing::info;

use wayland::views::LayerShellCanvasView;
use wayland::WaylandConnection;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = logging::init_logging()?;
    let mut conn = WaylandConnection::new().await?;

    let outputs = conn.outputs().collect::<Vec<_>>();
    if outputs.is_empty() {
        bail!("No displays found to output to.");
    }
    for output in outputs {
        info!("adding a new layer shell view!");
        let view = LayerShellCanvasView::new(
            &mut conn.state.shareable,
            &output,
            wayland::OverlayMode::Visible,
        )
        .await?;
        conn.state.views.add_canvas_view(view);
    }

    conn.run().await.context("test")?;
    Ok(())
}
