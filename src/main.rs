use duansheli::{DirConfig, declutter_directory};
use serde::Deserialize;
use core::fmt;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process;

struct CliArgs {
    filepath: String,
    dry_run: bool,
    print: bool
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

impl CliArgs {
    fn build(mut args: impl Iterator<Item = String>) -> CliArgs {
        args.next(); // skip binary name

        let mut filepath = None;
        let mut dry_run = false;
        let mut print = false;

        for arg in args {
            match arg.as_str() {
                "-n" | "--dry-run" => dry_run = true,
                "-p" | "--print" => print = true,
                _ => filepath = Some(arg),
            }
        }

        let filepath = filepath
            .unwrap_or_else(|| default_config_path().to_string_lossy().into_owned());
        CliArgs { filepath, dry_run, print }
    }
    
}

impl fmt::Display for CliArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "duansheli configuration")?;
        writeln!(f, "  config file : {}", self.filepath)?;
        write!(f, "  dry run     : {}", self.dry_run)
    }
}

fn main() {
    env_logger::init();

    let config_file = CliArgs::build(env::args());
    
    if let Err(e) = run(config_file) {
        log::error!("Application error: {e}");
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
            writeln!(f, "      archive after : {} hours", dir.time_to_archive_hours)?;
            writeln!(f, "      delete after  : {} hours", dir.time_to_deletion_hours)?;
        }
        Ok(())
    }
}

fn run(cli: CliArgs) -> Result<(), Box<dyn Error>> {
    if cli.print {
        println!("{cli}");
        match fs::read_to_string(&cli.filepath) {
            Ok(raw) => {
                let config: DuansheliConfig = toml::from_str(&raw)?;
                println!("{config}");
            }
            Err(e) => println!("  config      : not found ({e})"),
        }
        process::exit(0);
    }

    let config_raw = fs::read_to_string(&cli.filepath)?;
    log::info!("Config Filepath: {fp}", fp = &cli.filepath);
    let config: DuansheliConfig = toml::from_str(&config_raw)?;

    for dir_config in config.dirs {
        log::info!("Processing directory: {}", dir_config.path.display());
        declutter_directory(dir_config, cli.dry_run).unwrap_or_else(|err| {
            log::error!("Application error: {err}");
            process::exit(1);
        });
    }

    Ok(())
}
