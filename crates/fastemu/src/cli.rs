//! The command line: `fastemu [FILE...]` opens the window, `--version` and
//! `--help` print and exit. Subcommands go first; `run` starts the
//! headless runner, which parses its own arguments.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: fastemu [FILE...]
       fastemu run SUITE [OPTIONS]
       fastemu --version | --help

Opens the FastEmu window. Each .jar or .jad FILE is added to the library
and started.

Commands:
  run         Run a game without a window (fastemu run --help)

Options:
  -V, --version   Print the version and exit
  -h, --help      Print this help and exit
";

/// What the user asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the window, adding and starting `files`.
    Open {
        files: Vec<PathBuf>,
    },
    /// The headless runner, with its own arguments.
    Run {
        args: Vec<OsString>,
    },
    Version,
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    UnknownOption(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption(option) => write!(f, "unknown option {option}"),
        }
    }
}

/// Parses the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, CliError> {
    let mut args = args.into_iter().peekable();
    if args.peek().is_some_and(|a| a == "run") {
        args.next();
        return Ok(Command::Run {
            args: args.collect(),
        });
    }
    let mut files = Vec::new();
    let mut options_done = false;
    for arg in args {
        match arg.to_str() {
            Some("--") if !options_done => options_done = true,
            Some("-V" | "--version") if !options_done => return Ok(Command::Version),
            Some("-h" | "--help") if !options_done => return Ok(Command::Help),
            Some(option) if !options_done && option.starts_with('-') && option.len() > 1 => {
                return Err(CliError::UnknownOption(option.to_owned()));
            }
            _ => files.push(PathBuf::from(arg)),
        }
    }
    Ok(Command::Open { files })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Command, CliError> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_open_the_window() {
        assert_eq!(parse_strs(&[]), Ok(Command::Open { files: vec![] }));
    }

    #[test]
    fn files_are_opened() {
        assert_eq!(
            parse_strs(&["a.jar", "--", "-odd.jad"]),
            Ok(Command::Open {
                files: vec!["a.jar".into(), "-odd.jad".into()]
            })
        );
    }

    #[test]
    fn version_and_help_win() {
        assert_eq!(parse_strs(&["--version"]), Ok(Command::Version));
        assert_eq!(parse_strs(&["a.jar", "-V"]), Ok(Command::Version));
        assert_eq!(parse_strs(&["-h"]), Ok(Command::Help));
    }

    #[test]
    fn run_takes_the_rest() {
        assert_eq!(
            parse_strs(&["run", "game.jar", "--frames", "10"]),
            Ok(Command::Run {
                args: ["game.jar", "--frames", "10"]
                    .iter()
                    .map(OsString::from)
                    .collect()
            })
        );
        assert_eq!(
            parse_strs(&["game.jar", "run"]),
            Ok(Command::Open {
                files: vec!["game.jar".into(), "run".into()]
            }),
            "run is a command only in first place"
        );
    }

    #[test]
    fn unknown_options_are_errors() {
        assert_eq!(
            parse_strs(&["--fullscreen"]),
            Err(CliError::UnknownOption("--fullscreen".into()))
        );
    }
}
