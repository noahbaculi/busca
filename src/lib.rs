use core::panic;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub comparison_file_path: PathBuf,
    pub search_dir: String,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() - 1 < 2 {
            return Err("not enough arguments");
        }

        // The main comparison reference file is the first argument
        let mut comparison_file_path = PathBuf::new();
        comparison_file_path.push(&args[1]);
        if !comparison_file_path.is_file() {
            if !comparison_file_path.is_file() {
                return Err("The comparison file does not exist");
            }
        }

        // TODO make optional
        let search_dir = args[2].clone();

        Ok(Config {
            comparison_file_path,
            search_dir,
        })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    // let contents = fs::read_to_string(config.search_dir)?;

    // println!("With text:\n{contents}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_result() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }
}
