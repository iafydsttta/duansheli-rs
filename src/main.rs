use clap::{Parser, Subcommand};
use duansheli::{DirConfig, declutter_directory, default_log_path};
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process;

/// duansheli - directory declutter & archival tool
#[derive(Parser, Debug)]
#[command(name = "duansheli", version, about)]
struct Cli {
    /// Config file path [default: $XDG_CONFIG_HOME/duansheli/config.toml]
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Increase log verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Process directories: archive old files, delete ancient ones
    Run {
        /// Simulate actions without making changes
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Display the current configuration
    Print,
}

fn default_config_path() -> PathBuf {
    let config_home = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME environment variable not set");
            PathBuf::from(home).join(".config")
        });
    config_home.join("duansheli").join("config.toml")
}

fn stderr_level(verbose: u8) -> log::LevelFilter {
    match verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    }
}

fn init_logging(verbose: u8) -> Result<(), Box<dyn Error>> {
    let log_path = default_log_path();
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let file_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(log_file);

    let stderr_dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(stderr_level(verbose))
        .chain(std::io::stderr());

    fern::Dispatch::new()
        .chain(file_dispatch)
        .chain(stderr_dispatch)
        .apply()?;

    log::debug!("Logging to file: {}", log_path.display());
    Ok(())
}

fn init_stderr_only(verbose: u8) {
    let _ = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                record.level(),
                record.target(),
                message
            ))
        })
        .level(stderr_level(verbose))
        .chain(std::io::stderr())
        .apply();
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = init_logging(cli.verbose) {
        eprintln!("Warning: could not set up file logging: {e}");
        init_stderr_only(cli.verbose);
    }

    let config_path = cli.config.unwrap_or_else(default_config_path);

    let result = match cli.command {
        Some(Command::Run { dry_run }) => run_declutter(&config_path, dry_run),
        None => run_declutter(&config_path, false),
        Some(Command::Print) => print_config(&config_path),
    };

    if let Err(e) = result {
        log::error!("{e}");
        process::exit(1);
    }
}

#[derive(Deserialize)]
struct DuansheliConfig {
    dirs: Vec<DirConfig>,
}

impl fmt::Display for DuansheliConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  directories :")?;
        for dir in &self.dirs {
            writeln!(f, "    - {}", dir.path.display())?;
            writeln!(
                f,
                "      archive after : {} hours",
                dir.time_to_archive_hours
            )?;
            writeln!(
                f,
                "      delete after  : {} hours",
                dir.time_to_deletion_hours
            )?;
        }
        Ok(())
    }
}

fn print_config(config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("duansheli configuration");
    println!("  config file : {}", config_path.display());
    match fs::read_to_string(config_path) {
        Ok(raw) => {
            let config: DuansheliConfig = toml::from_str(&raw)?;
            println!("{config}");
        }
        Err(e) => println!("  config      : not found ({e})"),
    }
    Ok(())
}

fn run_declutter(config_path: &PathBuf, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let config_raw = fs::read_to_string(config_path)?;
    log::info!("Config Filepath: {}", config_path.display());
    let config: DuansheliConfig = toml::from_str(&config_raw)?;

    for dir_config in config.dirs {
        log::info!("Processing directory: {}", dir_config.path.display());
        declutter_directory(dir_config, dry_run)?;
    }

    Ok(())
}
