use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

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

fn get_num_shared_lines(ref_lines: &String, comp_lines: &String) -> i32 {
    // let mut text_differ = TextDiff::configure();
    // text_differ.newline_terminated(true);
    // text_differ = text_differ.newline_terminated(true);
    // let diff = text_differ.diff_lines(ref_lines, comp_lines);

    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    let mut num_shared_lines = 0;
    // Increment the number of shared lines for each equal line
    for change in diff.iter_all_changes() {
        // dbg!(change, change.missing_newline());
        // println!("---");
        match change.tag() {
            ChangeTag::Equal => num_shared_lines += 1,
            _ => (),
        };
    }

    num_shared_lines
}

fn get_perc_shared_lines(ref_file_path: &PathBuf, comp_file_path: &PathBuf) -> f32 {
    println!(
        "* Comparing reference file '{}' with '{}'.",
        &ref_file_path.display(),
        &comp_file_path.display()
    );

    // Read files
    let ref_lines = fs::read_to_string(ref_file_path).unwrap();
    let comp_lines = fs::read_to_string(comp_file_path).unwrap();

    let num_shared_lines = get_num_shared_lines(&ref_lines, &comp_lines);

    // If ref file ends with a newline, do not count it as line compared
    let mut num_ref_lines = ref_lines.split("\n").count();
    if ref_lines.ends_with('\n') {
        num_ref_lines -= 1;
    }

    num_shared_lines as f32 / num_ref_lines as f32
}

fn compare_with_file(ref_file_path: PathBuf, comp_file_path: PathBuf) -> HashMap<PathBuf, f32> {
    let mut path_to_perc_shared = HashMap::new();

    let perc_shared = get_perc_shared_lines(&ref_file_path, &comp_file_path);
    path_to_perc_shared.insert(comp_file_path, perc_shared);

    path_to_perc_shared
}

fn compare_with_dir(ref_file_path: PathBuf, comp_dir_path: PathBuf) -> HashMap<PathBuf, f32> {
    let mut path_to_perc_shared = HashMap::new();
    for dir_entry_result in WalkDir::new(&comp_dir_path.into_os_string().into_string().unwrap()) {
        let path_in_dir = dir_entry_result.unwrap().into_path();
        if !path_in_dir.is_file() {
            continue;
        }

        let perc_shared = get_perc_shared_lines(&ref_file_path, &path_in_dir);
        path_to_perc_shared.insert(path_in_dir.clone(), perc_shared);
    }

    path_to_perc_shared
}

pub fn run_search(ref_file_path: PathBuf, search: Search) -> Result<(), Box<dyn Error>> {
    let _perc_shared = match search.kind {
        SearchKind::File => compare_with_file(ref_file_path, search.path),
        SearchKind::Directory => compare_with_dir(ref_file_path, search.path),
    };

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn one_result() {
//         let query = "duct";
//         let contents = "\
// Rust:
// safe, fast, productive.
// Pick three.";

//         assert_eq!(vec!["safe, fast, productive."], run_search(query, contents));
//     }
// }
