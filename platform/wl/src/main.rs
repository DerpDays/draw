use std::{fs::File, os::unix::net::UnixListener, path::Path};

use async_executor::LocalExecutor;
// use async_executor::{Executor, LocalExecutor};
use clap::Parser;
use color_eyre::eyre::{Context, Result, eyre};
// use futures_lite::future;
use smithay_client_toolkit::reexports::calloop;

use crate::{
    cli::{Arguments, Command, IpcCommand, Message},
    wayland::WaylandConnection,
};

pub(crate) mod dirs;
pub(crate) mod wayland;

mod cli;
mod config;
mod logging;

pub const APP_NAME: &str = "draw";
pub const APP_SOCKET: &str = "draw.sock";
pub const APP_LOCK: &str = "draw.lock";

pub const APP_LOG_ENV: &str = "DRAW_LOG";

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Arguments::parse();
    let _guard = logging::init_logging()?;

    let lock_path = dirs::runtime_dir()?.join(APP_LOCK);
    let sock_path = dirs::runtime_dir()?.join(APP_SOCKET);

    let lock_file = File::create(lock_path).wrap_err("failed to create daemon lock file")?;

    // We create a lockfile which we use to check if there is an instance already running.
    if let Ok(_lock) = lock_file.try_lock() {
        create_daemon(args, sock_path)
    } else {
        handle_client(args, sock_path)
    }
}

// Parse command line options.
fn create_daemon<T: AsRef<Path>>(args: Arguments, sock_path: T) -> Result<()> {
    if let Some(cmd) = args.command {
        match cmd {
            Command::Msg { .. } | Command::Quit => {
                tracing::info!("No instance of draw is currently open!");
                return Ok(());
            }
        }
    }
    let outputs: Vec<String> = args.outputs;

    let mut conn = {
        let executor = LocalExecutor::new();
        futures_lite::future::block_on(executor.run(WaylandConnection::new()))
            .wrap_err("failed to create wayland connection")?
    };

    match std::fs::remove_file(&sock_path) {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        _ => Err(eyre!(
            "failed to remove the socket file before initialising"
        ))?,
    };
    // Insert IPC event handler.
    let listener =
        UnixListener::bind(sock_path).wrap_err("failed to bind listener to socket file")?;
    listener.set_nonblocking(true)?;

    let generic =
        calloop::generic::Generic::new(listener, calloop::Interest::READ, calloop::Mode::Level);
    // Insert into loop
    conn.event_loop
        .handle()
        .insert_source(generic, |_, listener, state| {
            // Accept new clients
            match listener.accept() {
                Ok((stream, _addr)) => {
                    tracing::info!("received new IPC connection.");
                    setup_client(stream, state.shareable.loop_handle.clone()).unwrap();
                    // Register client
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => tracing::error!("Accept error: {:?}", e),
            }

            Ok(calloop::PostAction::Continue)
        })?;

    if outputs.is_empty() {
        for (output, info) in conn.wayland().get_displays() {
            tracing::info!("adding a canvas for output name: {:?}", info.name);
            conn.state.add_canvas_output(output);
        }
    } else {
        for (_output, info) in conn.wayland().get_displays() {
            if let Some(name) = &info.name
                && outputs.contains(name)
            {
                // conn.add_canvas()
            }
        }
        println!("creating daemon on outputs: {outputs:?}");
    }
    conn.run().wrap_err("failed to run wayland connection")?;
    Ok(())
}

// Function to register a client stream
fn setup_client(
    stream: std::os::unix::net::UnixStream,
    handle: calloop::LoopHandle<'static, wayland::State>,
) -> Result<()> {
    stream.set_nonblocking(true)?;

    let generic =
        calloop::generic::Generic::new(stream, calloop::Interest::READ, calloop::Mode::Level);
    handle.insert_source(generic, |_, stream, state| {
        let res = bincode::decode_from_std_read::<IpcCommand, _, _>(
            &mut stream.as_ref(),
            bincode::config::standard(),
        );
        match res {
            Ok(command) => {
                let (_outputs, cmd) = command.into_tuple();
                tracing::info!("got a new that sent Command::{cmd:?}");
                match cmd {
                    Command::Quit => todo!(),
                    Command::Msg(message) => match message {
                        Message::Quit => {
                            tracing::info!("Removing all outputs");
                            state.canvas_outputs.clear();
                        }
                        Message::Open => {
                            for (output, info) in state.shareable.wayland.get_displays() {
                                tracing::info!("opening canvas on output: {:?}", info.name);
                                state.add_canvas_output(output);
                            }
                        }
                        Message::SaveCanvas { path: _ } => todo!(),
                        Message::ClearCanvas => todo!(),
                        Message::ToggleInteractivity => todo!(),
                        Message::SetInteractive => {
                            for view in state.canvas_outputs.values_mut() {
                                if let Err(err) = view.set_mode(
                                    &mut state.shareable,
                                    wayland::OverlayMode::Interactive,
                                ) {
                                    tracing::error!("failed to set mode to interactive: {err:?}");
                                };
                            }
                        }
                        Message::SetVisible => {
                            for view in state.canvas_outputs.values_mut() {
                                if let Err(err) = view
                                    .set_mode(&mut state.shareable, wayland::OverlayMode::Visible)
                                {
                                    tracing::error!("failed to set mode to visible: {err:?}");
                                };
                            }
                        }
                        Message::SetHidden => {
                            for view in state.canvas_outputs.values_mut() {
                                if let Err(err) = view
                                    .set_mode(&mut state.shareable, wayland::OverlayMode::Hidden)
                                {
                                    tracing::error!("failed to set mode to hidden: {err:?}");
                                };
                            }
                        }
                        Message::Draw(..) => todo!(),
                    },
                }
                Ok(calloop::PostAction::Continue)
            }
            Err(_) => {
                tracing::info!("closing IPC connection!");
                Ok(calloop::PostAction::Remove)
            }
        }
    })?;
    Ok(())
}

// Parse command line options.
fn handle_client<T: AsRef<Path>>(args: Arguments, sock_path: T) -> Result<()> {
    let mut stream = std::os::unix::net::UnixStream::connect(&sock_path)?;
    match args.command {
        Some(Command::Quit) => {
            bincode::encode_into_std_write(
                IpcCommand::new(args.outputs, Command::Quit),
                &mut stream,
                bincode::config::standard(),
            )?;
        }
        Some(Command::Msg(msg)) => {
            bincode::encode_into_std_write(
                IpcCommand::new(args.outputs, Command::Msg(msg)),
                &mut stream,
                bincode::config::standard(),
            )?;
        }
        None => {
            bincode::encode_into_std_write(
                IpcCommand::new(args.outputs, Command::Msg(Message::Open)),
                &mut stream,
                bincode::config::standard(),
            )?;
        }
    }

    println!("oneshot instance sent message");
    Ok(())
}
