use std::env;
use std::fs;
use std::process;
use std::error::Error;
// lib.rs
use serde::Deserialize;
use duansheli::{declutter_directory, DirConfig};

struct Config {
    filepath: String,
}

impl Config {
    fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str>{
        args.next();

        let filepath = match args.next() {
            Some(arg) => arg,
            None => return Err("Missing filepath"),
        };
        
        // let log_level = match args.next() {
        //     Some(arg) => arg,
        //     None => return Err("Missing log level")
        // };

        Ok(Config { filepath })
    }
}

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    if let Err(e) = run(config) {
        println!("Application error: {e}");
        process::exit(1);    
    }
}

#[derive(Deserialize)]
struct ConfigFile {
    dirs: Vec<DirConfig>
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let cfg_raw = fs::read_to_string(&config.filepath)?;
    println!("Config Filepath: {fp} \n", fp = &config.filepath);
    let config_file: ConfigFile = toml::from_str(&cfg_raw)?;

    println!("{:?}", config_file.dirs[0]);
    
    let target_path = "./fixtures/";

    let test_config = DirConfig{
        path: String::from(target_path),
        time_to_archive_hours: 42,
        time_to_delete_from_archive_hours: 42
    };

    declutter_directory(&target_path, test_config).unwrap_or_else(|err| {
        eprintln!("Application error: {err}");
        process::exit(1);
    });

    Ok(()) }
