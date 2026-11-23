//! DD-EN-003: the pure UCI protocol layer. No tokio, no process, no panics.

mod format;
mod lines;
mod options;
mod parse;

pub use format::{
    format_go, format_isready, format_position, format_quit, format_setoption, format_stop,
    format_uci, format_ucinewgame,
};
pub use lines::{LineSplitter, LineTooLong};
pub use options::{OptionKind, OptionSpec, OptionValue, SetOption, UciOptions};
pub use parse::{InfoLine, UciMessage, parse_line};
