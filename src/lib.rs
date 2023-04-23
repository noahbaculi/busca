use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

fn get_num_shared_lines(ref_lines: &String, comp_lines: &String) -> i32 {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    let mut num_shared_lines = 0;
    // Increment the number of shared lines for each equal line
    for change in diff.iter_all_changes() {
        // dbg!(change, change.missing_newline());
        // println!("---");

        if change.tag() == ChangeTag::Equal {
            num_shared_lines += 1
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
    // TODO reference lines should be read outside this repeated fn
    let ref_lines = fs::read_to_string(ref_file_path).unwrap();
    let comp_lines = fs::read_to_string(comp_file_path).unwrap();

    let num_shared_lines = get_num_shared_lines(&ref_lines, &comp_lines);

    // If ref file ends with a newline, do not count it as line compared
    let mut num_ref_lines = ref_lines.split('\n').count();
    if ref_lines.ends_with('\n') {
        num_ref_lines -= 1;
    }

    num_shared_lines as f32 / num_ref_lines as f32
}

pub fn run_search(
    ref_file_path: PathBuf,
    search_path: PathBuf,
) -> Result<HashMap<PathBuf, f32>, Box<dyn Error>> {
    let mut path_to_perc_shared: HashMap<PathBuf, f32> = HashMap::new();

    // Walk through search path
    for dir_entry_result in WalkDir::new(&search_path.into_os_string().into_string().unwrap()) {
        let path_in_dir = dir_entry_result.unwrap().into_path();

        // Skip paths that are not files
        if !path_in_dir.is_file() {
            continue;
        }

        let perc_shared = get_perc_shared_lines(&ref_file_path, &path_in_dir);
        path_to_perc_shared.insert(path_in_dir.clone(), perc_shared);
    }

    Ok(path_to_perc_shared)
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
