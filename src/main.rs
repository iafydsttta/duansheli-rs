use std::env;
use std::fs;
use std::process;
use std::error::Error;
// lib.rs
use duansheli::list_dir_with_meta;
use toml::Table;

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

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let cfg_raw = fs::read_to_string(&config.filepath)?;
    println!("Config Filepath: {fp} \n", fp = &config.filepath);
    // println!("Config: {cfg_raw}");

    let toml_table: Table = cfg_raw.parse().unwrap();
    // println!("{:?}", toml_table);
    let value = &toml_table["dirs"];

    let dir_configs = match value.as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            eprintln!("Not an array, or empty");
            process::exit(1);
        }
    };
    //
    // TODO: parse each array entry to class via #[derive(Deserialize)]: https://docs.rs/toml/latest/toml/

    println!("{:?}", dir_configs);

    let target_path = "./fixtures/";
    list_dir_with_meta(&target_path).unwrap_or_else(|err| {
        eprintln!("Application error: {err}");
        process::exit(1);
    });

    Ok(()) }
