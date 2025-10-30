use std::path::PathBuf;

use bincode::{Decode, Encode};
use clap::{ArgAction, Args, Parser, Subcommand};
use euclid::default::Point2D;

// TODO: completions with clap-complete

#[derive(Debug, Parser)]
#[command(version, about = "A highly configurable annotation tool.", long_about = None)]
pub struct Arguments {
    /// Sets a custom config location
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Start the canvas from the given save file.
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Apply the command to only the given outputs (comma delimited).
    #[arg(short, long, action=ArgAction::Set, value_delimiter=',', global = true)]
    pub outputs: Vec<String>,
}

#[derive(Clone, Debug, Decode, Encode)]
pub struct IpcCommand {
    outputs: Vec<String>,
    command: Command,
}
impl IpcCommand {
    pub fn new(outputs: Vec<String>, command: Command) -> Self {
        Self { outputs, command }
    }
    pub fn into_tuple(self) -> (Vec<String>, Command) {
        (self.outputs, self.command)
    }
}

#[derive(Clone, Debug, Decode, Encode, Subcommand)]
#[command(rename_all = "snake_case")]
#[command(disable_help_subcommand = true)]
pub enum Command {
    Quit,
    /// Send commands to the currently running canvas instsance
    #[command(subcommand)]
    Msg(Message),
}

#[derive(Clone, Debug, Decode, Encode, Subcommand)]
#[command(rename_all = "snake_case")]
#[command(disable_help_subcommand = true)]
pub enum Message {
    /// Close the canvas without saving.
    Quit,
    /// Open a new canvas on the given outputs, only works if the daemon is currently running.
    Open,

    SaveCanvas {
        path: Option<PathBuf>,
    },
    ClearCanvas,

    ToggleInteractivity,

    /// Set the canvas mode on the selected outputs as interactive.
    SetInteractive,
    /// Set the canvas mode on the selected outputs as visible (non-interactive).
    SetVisible,
    /// Set the canvas mode on the selected outputs as hidden.
    SetHidden,

    /// Execute a given draw command.
    #[command(subcommand)]
    Draw(DrawCommand),
}

// TODO: add all of these draw commands
#[derive(Clone, Debug, Decode, Encode, Subcommand)]
pub enum DrawCommand {
    Pen(PenArgs),
    Line,
    Arrow,
    Rectangle,
    Ellipse,
    Text,
    Highlighter,
    Eraser(PositionArg),
}

#[derive(Args, Clone, Debug, Decode, Encode)]
pub struct PenArgs {
    /// All of the points that the highlighter will visit, in the format x.x,y.y
    #[arg(required = true, last=true, value_parser= clap::value_parser!(PositionArg), num_args = 2..)]
    points: Vec<PositionArg>,
}

#[derive(Args, Clone, Copy, Debug, Decode, Encode)]
pub struct PositionArg {
    x: f32,
    y: f32,
}

impl std::str::FromStr for PositionArg {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(',').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(format!("expected point format: x.x,y.y"));
        }
        Ok(PositionArg {
            x: parts[0].parse::<f32>().map_err(|e| e.to_string())?,
            y: parts[1].parse::<f32>().map_err(|e| e.to_string())?,
        })
    }
}

impl From<PositionArg> for Point2D<f32> {
    fn from(value: PositionArg) -> Self {
        Self::new(value.x, value.y)
    }
}
