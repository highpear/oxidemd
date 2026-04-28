use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::export::write_html_export;

#[derive(Debug)]
pub enum CliAction {
    RunGui {
        initial_file: Option<PathBuf>,
        restore_file: bool,
        reset_session: bool,
    },
    PrintHelp,
    PrintVersion,
    ExportHtml {
        input: PathBuf,
        output: PathBuf,
    },
}

pub struct GuiLaunchOptions {
    pub initial_file: Option<PathBuf>,
    pub restore_file: bool,
    pub reset_session: bool,
}

pub fn parse_args<I>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();

    match args.as_slice() {
        [] => Ok(CliAction::RunGui {
            initial_file: None,
            restore_file: true,
            reset_session: false,
        }),
        [flag] if flag == OsStr::new("--help") || flag == OsStr::new("-h") => {
            Ok(CliAction::PrintHelp)
        }
        [flag] if flag == OsStr::new("--version") || flag == OsStr::new("-V") => {
            Ok(CliAction::PrintVersion)
        }
        [flag] if flag == OsStr::new("--no-restore-file") => Ok(CliAction::RunGui {
            initial_file: None,
            restore_file: false,
            reset_session: false,
        }),
        [flag] if flag == OsStr::new("--reset-session") => Ok(CliAction::RunGui {
            initial_file: None,
            restore_file: false,
            reset_session: true,
        }),
        [flag, initial_file]
            if flag == OsStr::new("--no-restore-file") && !looks_like_flag(initial_file) =>
        {
            Ok(CliAction::RunGui {
                initial_file: Some(PathBuf::from(initial_file)),
                restore_file: false,
                reset_session: false,
            })
        }
        [flag, initial_file]
            if flag == OsStr::new("--reset-session") && !looks_like_flag(initial_file) =>
        {
            Ok(CliAction::RunGui {
                initial_file: Some(PathBuf::from(initial_file)),
                restore_file: false,
                reset_session: true,
            })
        }
        [flag, input, output] if flag == OsStr::new("--export-html") => {
            Ok(CliAction::ExportHtml {
                input: PathBuf::from(input),
                output: PathBuf::from(output),
            })
        }
        [initial_file] if !looks_like_flag(initial_file) => {
            Ok(CliAction::RunGui {
                initial_file: Some(PathBuf::from(initial_file)),
                restore_file: true,
                reset_session: false,
            })
        }
        [flag] if flag == OsStr::new("--export-html") => Err(
            "Missing arguments for --export-html. Usage: oxidemd --export-html <input.md> <output.html>"
                .to_owned(),
        ),
        [flag, _] if flag == OsStr::new("--export-html") => Err(
            "Missing output file for --export-html. Usage: oxidemd --export-html <input.md> <output.html>"
                .to_owned(),
        ),
        _ => Err("Unsupported arguments. Use --help for usage.".to_owned()),
    }
}

pub fn run_cli_action(action: CliAction) -> Result<GuiLaunchOptions, i32> {
    match action {
        CliAction::RunGui {
            initial_file,
            restore_file,
            reset_session,
        } => Ok(GuiLaunchOptions {
            initial_file,
            restore_file,
            reset_session,
        }),
        CliAction::PrintHelp => {
            println!("{}", help_text());
            Err(0)
        }
        CliAction::PrintVersion => {
            println!("OxideMD {}", env!("CARGO_PKG_VERSION"));
            Err(0)
        }
        CliAction::ExportHtml { input, output } => match write_html_export(&input, &output) {
            Ok(()) => {
                println!("Exported: {}", output.display());
                Err(0)
            }
            Err(error) => {
                eprintln!("Failed to export: {}", error);
                Err(1)
            }
        },
    }
}

fn looks_like_flag(value: &OsStr) -> bool {
    value
        .to_str()
        .map(|value| value.starts_with('-'))
        .unwrap_or(false)
}

fn help_text() -> &'static str {
    concat!(
        "OxideMD\n\n",
        "Usage:\n",
        "  oxidemd [file.md]\n",
        "  oxidemd --no-restore-file [file.md]\n",
        "  oxidemd --reset-session [file.md]\n",
        "  oxidemd --export-html <input.md> <output.html>\n",
        "  oxidemd --help\n",
        "  oxidemd --version\n\n",
        "Options:\n",
        "  --no-restore-file  Start without reopening the previous file.\n",
        "  --reset-session    Start with default settings and clear saved recent files.\n",
        "  --export-html      Export a Markdown file as HTML without opening the GUI.\n",
        "  -h, --help         Show this help text.\n",
        "  -V, --version      Show the OxideMD version.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::{CliAction, parse_args};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_gui_without_file() {
        let action = parse_args(args(&[])).expect("args should parse");

        assert!(matches!(
            action,
            CliAction::RunGui {
                initial_file: None,
                restore_file: true,
                reset_session: false
            }
        ));
    }

    #[test]
    fn parses_gui_with_file() {
        let action = parse_args(args(&["sample.md"])).expect("args should parse");

        assert!(
            matches!(action, CliAction::RunGui { initial_file: Some(path), restore_file: true, reset_session: false } if path == PathBuf::from("sample.md"))
        );
    }

    #[test]
    fn parses_gui_without_restored_file() {
        let action = parse_args(args(&["--no-restore-file"])).expect("args should parse");

        assert!(matches!(
            action,
            CliAction::RunGui {
                initial_file: None,
                restore_file: false,
                reset_session: false
            }
        ));
    }

    #[test]
    fn parses_gui_with_file_without_restored_file() {
        let action =
            parse_args(args(&["--no-restore-file", "sample.md"])).expect("args should parse");

        assert!(
            matches!(action, CliAction::RunGui { initial_file: Some(path), restore_file: false, reset_session: false } if path == PathBuf::from("sample.md"))
        );
    }

    #[test]
    fn parses_gui_with_reset_session() {
        let action = parse_args(args(&["--reset-session"])).expect("args should parse");

        assert!(matches!(
            action,
            CliAction::RunGui {
                initial_file: None,
                restore_file: false,
                reset_session: true
            }
        ));
    }

    #[test]
    fn parses_gui_with_file_and_reset_session() {
        let action =
            parse_args(args(&["--reset-session", "sample.md"])).expect("args should parse");

        assert!(
            matches!(action, CliAction::RunGui { initial_file: Some(path), restore_file: false, reset_session: true } if path == PathBuf::from("sample.md"))
        );
    }

    #[test]
    fn parses_html_export() {
        let action = parse_args(args(&["--export-html", "input.md", "output.html"]))
            .expect("args should parse");

        assert!(matches!(
            action,
            CliAction::ExportHtml { input, output }
                if input == PathBuf::from("input.md") && output == PathBuf::from("output.html")
        ));
    }

    #[test]
    fn rejects_incomplete_html_export() {
        let error =
            parse_args(args(&["--export-html", "input.md"])).expect_err("args should be rejected");

        assert!(error.contains("Missing output file"));
    }
}
