use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

fn get_num_shared_lines(ref_lines: &str, comp_lines: &str) -> i32 {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    let mut num_shared_lines = 0;
    // Increment the number of shared lines for each equal line
    // TODO try functional pattern
    for change in diff.iter_all_changes() {
        // dbg!(change, change.missing_newline());
        // println!("---");

        if change.tag() == ChangeTag::Equal {
            num_shared_lines += 1
        };
    }

    num_shared_lines
}

fn get_perc_shared_lines(ref_lines: &str, comp_file_path: &PathBuf) -> f32 {
    // Read files
    let comp_lines = fs::read_to_string(comp_file_path).unwrap();

    let num_shared_lines = get_num_shared_lines(ref_lines, &comp_lines);

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

    let ref_lines = fs::read_to_string(ref_file_path).unwrap();

    // Walk through search path
    for dir_entry_result in WalkDir::new(&search_path.into_os_string().into_string().unwrap()) {
        let path_in_dir = dir_entry_result?.into_path();

        // Skip paths that are not files
        if !path_in_dir.is_file() {
            continue;
        }

        let perc_shared = get_perc_shared_lines(&ref_lines, &path_in_dir);
        path_to_perc_shared.insert(path_in_dir.clone(), perc_shared);
    }

    Ok(path_to_perc_shared)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_num_shared_lines_standard() {
        let ref_lines = "2\n4\n";
        let comp_lines = "1\n2\n3\n4\n5\n";

        assert_eq!(get_num_shared_lines(ref_lines, comp_lines), 2);
    }
}