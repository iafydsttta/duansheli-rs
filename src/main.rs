use std::env;
use std::fs;
use std::process;
use std::error::Error;
// lib.rs
use duansheli::list_dir_with_meta;
use serde::Deserialize;

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
#[derive(Deserialize, Debug)]
struct DirConfig {
    path: String,
    time_to_archive_hours: i32,
    time_to_delete_from_archive_hours: i32
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let cfg_raw = fs::read_to_string(&config.filepath)?;
    println!("Config Filepath: {fp} \n", fp = &config.filepath);
    let config_file: ConfigFile = toml::from_str(&cfg_raw)?;

    println!("{:?}", config_file.dirs[0]);
    
    let target_path = "./fixtures/";
    list_dir_with_meta(&target_path).unwrap_or_else(|err| {
        eprintln!("Application error: {err}");
        process::exit(1);
    });

    Ok(()) }
