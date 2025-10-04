use any_spawner::Executor;
use euclid::default::Point2D;
use gui_reactive::{
    events::EventPhase,
    tree::{ElementChild, Node, NodeVisitor, StyleWrapper},
    widgets::primitives::{div, text},
    Tree, TreeManager,
};
use input::{MouseEvent, MouseEventKind};
use reactive_graph::{
    owner::Owner,
    signal::signal,
    traits::{GetUntracked, Set},
};
use taffy::{
    prelude::{length, percent, TaffyMaxContent},
    Size, Style,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct Visitor;
impl NodeVisitor for Visitor {
    fn visit<N: Node>(&mut self, node: &N) {
        println!("visiting: {}", node.debug_label(),);
    }
}

#[tokio::main]
async fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::from_env("GUI_LOG"))
        .with(fmt_layer)
        .init();

    let mgr = TreeManager::new(
        || {
            tracing::info!("called redraw now!");
        },
        |duration| {
            tracing::info!("called redraw duration {:?}!", duration);
        },
    );
    Executor::init_tokio().expect("failed to init executor");
    let mut tree = Tree::build(Size::MAX_CONTENT, mgr, || {
        let (texta, set_texta) = signal("initial".to_string());

        div()
            .style(StyleWrapper::default())
            .child(
                text(texta)
                    .style(Style {
                        size: length(100.),
                        ..Style::DEFAULT
                    })
                    .child(
                        div()
                            .style(Style {
                                size: length(20.),
                                ..Style::DEFAULT
                            })
                            .on_mouse(move |a, ctx| {
                                if ctx.current_phase() == EventPhase::AtTarget {
                                    if matches!(ctx.payload().kind, MouseEventKind::Press { .. }) {
                                        set_texta.set("new".to_string());
                                        tracing::info!("got mouse click");
                                    }
                                }
                            }),
                    ),
            )
            .child(text("child2").style(Style {
                size: percent(1.),
                ..Style::DEFAULT
            }))
    });

    tree.owner
        .clone()
        .with(|| async {
            // for _ in 0..10 {
            //     Executor::tick().await;
            // }
            tracing::info!("current_owner: {:?}", Owner::current());

            tree.compute_root_layout();
            // tracing::debug!("layout tree: {:#?}", tree.layout_tree);
            // taffy::print_tree(&tree, tree.root_node());

            tracing::info!(
                "source value before: {:?}",
                tree.inner.children.0.inner.text.get_untracked()
            );
            tree.on_mouse(MouseEvent::new(
                Point2D::new(50., 50.),
                input::MouseEventKind::Enter,
            ));
            tree.on_mouse(MouseEvent::new(
                Point2D::new(10., 10.),
                input::MouseEventKind::Motion { time: 0 },
            ));
            tree.on_mouse(MouseEvent::new(
                Point2D::new(10., 10.),
                input::MouseEventKind::Press {
                    time: 0,
                    button: input::MouseButton::Left,
                },
            ));
            Executor::tick().await;
            // before rendering call:
            tree.relayout_nodes();

            tracing::info!(
                "source value after: {:?}",
                tree.inner.children.0.inner.text.get_untracked()
            );
        })
        .await
    // })
    // .await;
}
