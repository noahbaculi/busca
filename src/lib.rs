use std::error::Error;
// use std::fs;
use std::path::PathBuf;

pub fn search(comparison_file_path: PathBuf, search_dir: PathBuf) -> Result<(), Box<dyn Error>> {
    // let contents = fs::read_to_string(config.search_dir)?;

    println!(
        "{} | {}",
        comparison_file_path.display(),
        search_dir.display()
    );

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
