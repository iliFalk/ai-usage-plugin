//! `ai-usagebar` binary. The library does all the work — this is just the
//! tokio bootstrap + clap parse.

use ai_usagebar::cli::{Cli, Command};
use clap::Parser;

fn main() {
    let (argv, config_path) = match split_config_arg(std::env::args_os()) {
        Ok(split) => split,
        Err(message) => {
            eprintln!("ai-usagebar: {message}");
            std::process::exit(2);
        }
    };
    // Parse first so `--version`/`--help` self-report regardless of the
    // `--config` file's state; only then validate the override path, which
    // must be set before any config is read.
    let cli = Cli::parse_from(argv);
    if let Some(path) = &config_path {
        if !path.is_file() {
            eprintln!(
                "ai-usagebar: config file not found: {} (create it first, or point --config at an existing file)",
                path.display()
            );
            std::process::exit(2);
        }
        ai_usagebar::config::set_override_path(path);
    }

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(error) => {
            eprintln!("ai-usagebar: failed to create runtime: {error}");
            std::process::exit(1);
        }
    };
    let code = match cli.command {
        Some(Command::Usage { json }) => rt.block_on(ai_usagebar::report::run(json)),
        Some(Command::Settings { action }) => ai_usagebar::settings::run_cli(&action),
        None => {
            eprintln!("ai-usagebar: a command is required; try `ai-usagebar usage --json`");
            2
        }
    };
    std::process::exit(code);
}

/// Extract `--config <PATH>` (or `--config=PATH`) from argv before clap sees
/// it, so the flag is accepted in any position — including alongside a
/// subcommand. Returns the remaining argv (with the program name kept first)
/// and the override, if any. A bare `--` ends the scan: it and everything after
/// it are positional (clap convention) and pass through untouched.
fn split_config_arg<I>(
    argv: I,
) -> Result<(Vec<std::ffi::OsString>, Option<std::path::PathBuf>), String>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    let mut argv = argv.into_iter();
    let mut rest = Vec::new();
    let mut config: Option<std::path::PathBuf> = None;
    if let Some(program) = argv.next() {
        rest.push(program);
    }
    while let Some(arg) = argv.next() {
        if arg == "--" {
            rest.push(arg);
            rest.extend(argv);
            break;
        }
        // Compare at the OsStr level so a path the platform can store but
        // UTF-8 cannot represent is never mangled by a lossy conversion.
        let value = if arg == "--config" {
            match argv.next() {
                Some(value) => Some(std::path::PathBuf::from(value)),
                None => return Err("--config requires a path".into()),
            }
        } else {
            ai_usagebar::config::config_flag_value(&arg)
        };
        if let Some(value) = value {
            if config.is_some() {
                return Err("--config given more than once".into());
            }
            config = Some(value);
        } else {
            rest.push(arg);
        }
    }
    Ok((rest, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<std::ffi::OsString> {
        items.iter().map(std::ffi::OsString::from).collect()
    }

    #[test]
    fn flag_is_extracted_before_the_subcommand() {
        let (rest, config) = split_config_arg(argv(&[
            "ai-usagebar",
            "--config",
            "a.toml",
            "usage",
            "--json",
        ]))
        .unwrap();
        assert_eq!(config.as_deref(), Some(std::path::Path::new("a.toml")));
        assert_eq!(rest, argv(&["ai-usagebar", "usage", "--json"]));
    }

    #[test]
    fn equals_form_is_extracted_after_the_subcommand() {
        let (rest, config) =
            split_config_arg(argv(&["ai-usagebar", "usage", "--config=a.toml", "--json"])).unwrap();
        assert_eq!(config.as_deref(), Some(std::path::Path::new("a.toml")));
        assert_eq!(rest, argv(&["ai-usagebar", "usage", "--json"]));
    }

    #[test]
    fn tokens_after_double_dash_pass_through_untouched() {
        let (rest, config) =
            split_config_arg(argv(&["ai-usagebar", "--", "--config", "a.toml"])).unwrap();
        assert!(
            config.is_none(),
            "`--config` behind `--` is a positional, not the flag"
        );
        assert_eq!(rest, argv(&["ai-usagebar", "--", "--config", "a.toml"]));
    }

    #[test]
    fn duplicate_flag_is_rejected_in_every_form() {
        for repeated in [
            &["ai-usagebar", "--config", "a.toml", "--config", "b.toml"][..],
            &["ai-usagebar", "--config=a.toml", "--config=b.toml"][..],
            &["ai-usagebar", "--config", "a.toml", "--config=b.toml"][..],
        ] {
            let err = split_config_arg(argv(repeated)).unwrap_err();
            assert_eq!(err, "--config given more than once");
        }
    }

    #[test]
    fn dangling_flag_is_rejected() {
        let err = split_config_arg(argv(&["ai-usagebar", "usage", "--config"])).unwrap_err();
        assert_eq!(err, "--config requires a path");
    }

    /// Both `--config` forms must hand the exact platform bytes to the path.
    #[cfg(unix)]
    #[test]
    fn both_forms_keep_undecodable_bytes_intact() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let equals = std::ffi::OsString::from_vec(b"--config=caf\xe9.toml".to_vec());
        let (_rest, config) =
            split_config_arg(vec![std::ffi::OsString::from("ai-usagebar"), equals]).unwrap();
        assert_eq!(
            config.expect("equals form parsed").as_os_str().as_bytes(),
            b"caf\xe9.toml"
        );
        let spaced = std::ffi::OsString::from_vec(b"caf\xe9.toml".to_vec());
        let (_rest, config) = split_config_arg(vec![
            std::ffi::OsString::from("ai-usagebar"),
            std::ffi::OsString::from("--config"),
            spaced,
        ])
        .unwrap();
        assert_eq!(
            config.expect("spaced form parsed").as_os_str().as_bytes(),
            b"caf\xe9.toml"
        );
    }
}
