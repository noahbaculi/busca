use std::error::Error;
// use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub enum SearchKind {
    File,
    Directory,
}

#[derive(Debug)]
pub struct Search {
    pub path: PathBuf,
    pub kind: SearchKind,
}

pub fn run_search(ref_file_path: PathBuf, search: Search) -> Result<(), Box<dyn Error>> {
    // let contents = fs::read_to_string(config.search_dir)?;

    println!("{} | {:?}", ref_file_path.display(), search);

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

        assert_eq!(vec!["safe, fast, productive."], run_search(query, contents));
    }
}
