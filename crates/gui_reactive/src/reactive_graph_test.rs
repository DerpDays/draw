use std::sync::{Arc, OnceLock};

use any_spawner::Executor;
use reactive_graph::{
    effect::RenderEffect,
    owner::Owner,
    signal::signal,
    traits::{Get, GetUntracked, Set},
    wrappers::read::Signal,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct RedrawData {
    now_fn: fn(),
}

pub struct RedrawHandler(std::sync::OnceLock<RedrawData>);
impl RedrawHandler {
    const fn new() -> Self {
        Self(OnceLock::new())
    }

    pub fn try_init(&self, now_fn: fn()) -> Option<()> {
        self.0.set(RedrawData { now_fn }).ok()
    }

    fn get(&self) -> &RedrawData {
        self.0
            .get()
            .expect("RedrawHandler must be initialised before the tree.")
    }

    pub fn now(&self) {
        (self.get().now_fn)();
    }
}

pub static REDRAW_HANDLER: RedrawHandler = RedrawHandler::new();

pub struct Container {
    text: Signal<String>,
    _effect: RenderEffect<()>,
    click_fn: EventHandler,
}

pub struct EventHandler {
    handler: Arc<dyn Fn() + Send + Sync>,
}
impl EventHandler {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(f),
        }
    }
    pub(crate) fn handle(&self) {
        (self.handler)()
    }
}

fn container<F>(sig: impl Into<Signal<String>>, click_fn: F) -> Container
where
    F: Fn() + Send + Sync + 'static,
{
    let sig = sig.into();
    let _effect = RenderEffect::new(move |_| {
        sig.get();
        REDRAW_HANDLER.now();
    });
    Container {
        text: sig,
        _effect,
        click_fn: EventHandler::new(click_fn),
    }
}

// #[tokio::main]
// async fn main() {
//     let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
//     tracing_subscriber::registry()
//         .with(tracing_subscriber::filter::EnvFilter::from_env("TEST_LOG"))
//         .with(fmt_layer)
//         .init();
//
//     REDRAW_HANDLER.try_init(|| {
//         tracing::warn!("called redraw now!!!!!!");
//     });
//     Executor::init_tokio().expect("failed to init executor");
//     let local = tokio::task::LocalSet::new();
//     local
//         .run_until(async {
//             tracing::info!("creating owner");
//             let owner = Owner::new();
//             owner
//                 .with(|| async {
//                     tracing::info!("creating container");
//                     let container = {
//                         let (sig, set_sig) = signal("testing".to_string());
//                         container(sig, move || {
//                             set_sig.set("new".to_string());
//                         })
//                     };
//                     tracing::info!("ticking");
//                     Executor::tick().await;
//                     tracing::info!("calling click_fn");
//                     container.click_fn.handle();
//                     tracing::info!("ticking");
// for _ in 0..10 {
//     Executor::tick().await;
// }
//                     tracing::info!("printing");
//                     tracing::info!("value is: {:?}", container.text.get_untracked());
//                 })
//                 .await;
//         })
//         .await;
// }

#[tokio::main]
async fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::from_env("TEST_LOG"))
        .with(fmt_layer)
        .init();

    REDRAW_HANDLER.try_init(|| {
        tracing::warn!("called redraw now!!!!!!");
    });
    Executor::init_tokio().expect("failed to init executor");
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            tracing::info!("creating owner");
            let owner = Owner::new();
            let container = owner.with(|| {
                tracing::info!("creating container");
                {
                    let (sig, set_sig) = signal("testing".to_string());
                    container(sig, move || {
                        set_sig.set("new".to_string());
                    })
                }
            });

            for _ in 0..10 {
                Executor::tick().await;
            }

            owner.with(|| {
                container.click_fn.handle();
            });
            for _ in 0..10 {
                Executor::tick().await;
            }

            owner.with(|| {
                tracing::info!("value is: {:?}", container.text.get_untracked());
            });
            // owner
            //     .with(|| async {
            //         tracing::info!("ticking");
            //         Executor::tick().await;
            //         tracing::info!("calling click_fn");
            //         container.click_fn.handle();
            //         tracing::info!("ticking");
            //         for _ in 0..10 {
            //             Executor::tick().await;
            //         }
            //         tracing::info!("printing");
            //         tracing::info!("value is: {:?}", container.text.get_untracked());
            //     })
            //     .await;
        })
        .await;
}
