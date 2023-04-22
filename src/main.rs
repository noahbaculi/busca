use std::env;
use std::process;

use busca::Config;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    dbg!(&config);

    println!("Searching for {}", config.comparison_file_path.display());
    println!("In file {}", config.search_dir);

    if let Err(e) = busca::run(config) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
