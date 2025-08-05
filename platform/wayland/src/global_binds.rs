use anyhow::{Context, Result};
use ashpd::{
    desktop::{
        global_shortcuts::{GlobalShortcuts, NewShortcut},
        Session,
    },
    register_host_app, AppID,
};
use futures_util::{stream::select_all, Stream, StreamExt};
use smithay_client_toolkit::reexports::calloop;
use std::{marker::PhantomData, str::FromStr};
use strum::{Display as StrumDisplay, EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use tracing::{error, trace, trace_span, warn};

#[derive(Debug)]
pub enum ShortcutEvents {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone, EnumString, IntoStaticStr, EnumIter, StrumDisplay)]
#[strum(serialize_all = "snake_case")]
pub enum Shortcuts {
    Toggle,
    Interact,
}

impl Shortcuts {
    pub const fn description(&self) -> &'static str {
        match self {
            Shortcuts::Toggle => "Toggle the interactivity of the annotation panel.",
            Shortcuts::Interact => "Enable interactivity on the annotation panel whilst held.",
        }
    }

    fn all_shortcuts() -> Vec<(String, String)> {
        Self::iter()
            .map(|s| {
                let description = s.description().into();
                (s.to_string(), description)
            })
            .collect()
    }
}

pub struct WaylandKeybinds<'a, Status> {
    instance: GlobalShortcuts<'a>,
    session: Session<'a, GlobalShortcuts<'a>>,
    status: PhantomData<Status>,
}

pub struct Bindable;
pub struct Bound;

impl<'a> WaylandKeybinds<'a, ()> {
    pub async fn new_session() -> Result<WaylandKeybinds<'a, Bindable>> {
        trace!("Creating new global shortcuts instance");
        // FIXME: register host app
        //
        let app_id = AppID::from_str("com.drawthings.app").context("invalid app_id")?;
        if let Err(e) = register_host_app(app_id).await {
            warn!(
                "Failed to register host app id, this is likely due to missing the corresponding .desktop file\n\nGlobal keybinds may still be bound possibly under a different default app id.\nFull error:\n{e}"
            );
        }
        let instance = GlobalShortcuts::new()
            .await
            .context("failed to create a new global shortcuts instance")?;
        trace!("Attempting to create a shortcut session");
        let session = instance
            .create_session()
            .await
            .context("failed to create shortcut session")?;
        trace!("Created shortcut session");

        Ok(WaylandKeybinds {
            instance,
            session,
            status: PhantomData,
        })
    }

    pub async fn build_source() -> Result<calloop::channel::Channel<(ShortcutEvents, Shortcuts)>> {
        Self::new_session()
            .await
            .context("failed to create new shortcut session")?
            .bind()
            .await
            .context("failed to bind global keybinds")?
            .to_event_source()
            .await
            .context("failed to create event source")
    }
}

impl<'a> WaylandKeybinds<'a, Bindable> {
    /// Register the global keybinds with the running xdg-desktop-portal
    pub async fn bind(self) -> Result<WaylandKeybinds<'a, Bound>> {
        trace_span!("binding linux global keybinds");
        let shortcuts = Shortcuts::all_shortcuts();

        // FIXME: getting the response for current shortcuts now seems to fail, this should be reported
        // upstream. This doesn't affect much, since it is just to prevent binding already bound keybinds,
        // which might show a prompt again or cause an error in some portal implementations.
        // This needs testing to check whether this is due to asphd or my current compositor.
        //
        // trace!("fetching already bound global keybinds");
        // // list all shortcuts to check if they have been already registered.
        // let list_response = self
        //     .instance
        //     .list_shortcuts(&self.session)
        //     .await
        //     .context("failed to request to list existing global keybinds")?
        //     .response()
        //     .context("failed to list all existing global keybinds")?;
        // let existing = list_response.shortcuts();
        // trace!("filtering out keybinds that already are registered");
        // shortcuts.retain(|(id, _desc)| !existing.iter().any(|shortcut| *id == shortcut.id()));

        let to_bind = shortcuts
            .into_iter()
            .map(|(id, desc)| NewShortcut::new(id, desc))
            .collect::<Vec<_>>();

        self.instance
            .bind_shortcuts(&self.session, to_bind.as_slice(), None)
            .await?
            .response()
            .context("failed to bind shortcuts")?;
        //
        trace!("binded successfully");
        Ok(WaylandKeybinds {
            instance: self.instance,
            session: self.session,
            status: PhantomData,
        })
    }
}
impl<'a> WaylandKeybinds<'a, Bound> {
    async fn get_pressed_stream(
        &self,
    ) -> Option<impl Stream<Item = (ShortcutEvents, Shortcuts)> + use<>> {
        Some(
            self.instance
                .receive_activated()
                .await
                .ok()?
                .filter_map(|x| async move {
                    Shortcuts::from_str(x.shortcut_id())
                        .ok()
                        .map(|x| (ShortcutEvents::Pressed, x))
                }),
        )
    }

    async fn get_released_stream(
        &self,
    ) -> Option<impl Stream<Item = (ShortcutEvents, Shortcuts)> + use<>> {
        Some(
            self.instance
                .receive_deactivated()
                .await
                .ok()?
                .filter_map(|x| async move {
                    Shortcuts::from_str(x.shortcut_id())
                        .ok()
                        .map(|x| (ShortcutEvents::Released, x))
                }),
        )
    }

    pub async fn to_event_source(
        &self,
    ) -> Result<calloop::channel::Channel<(ShortcutEvents, Shortcuts)>> {
        trace!("getting keybind and connection streams");

        let pressed = self
            .get_pressed_stream()
            .await
            .context("failed to get activated stream")?
            .boxed();

        let released = self
            .get_released_stream()
            .await
            .context("failed to get deactivated stream")?
            .boxed();

        let mut merged = select_all(vec![pressed, released]);

        let (tx, chan) = calloop::channel::channel::<(ShortcutEvents, Shortcuts)>();

        tokio::spawn(async move {
            while let Some(ev) = merged.next().await {
                if let Err(e) = tx.send(ev) {
                    error!("failed to send global keybind event to calloop channel: {e}");
                    break;
                }
            }
        });

        Ok(chan)
    }
}
