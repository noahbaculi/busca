use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Returns the number of lines from `ref_lines` that also exist in `comp_lines`.
///
/// Note: Final new lines are included in the diff comparisons.
///
/// # Examples
///
/// ```
/// //                ✓   ✓  x   ✓   x      = 3
/// let ref_lines = "12\n14\n5\n17\n19\n";
/// let comp_lines = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_num_shared_lines(ref_lines, comp_lines);
/// assert_eq!(result, 3);
/// ```
/// ---
/// ```
/// //                ✓   ✓  x   x    = 2
/// let ref_lines = "12\n14\n5\n17";
/// let comp_lines = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_num_shared_lines(ref_lines, comp_lines);
/// assert_eq!(result, 2);
/// ```
///
pub fn get_num_shared_lines(ref_lines: &str, comp_lines: &str) -> i32 {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    // let mut num_shared_lines = 0;
    // Increment the number of shared lines for each equal line
    let num_shared_lines = diff
        .iter_all_changes()
        .filter(|change| change.tag() == ChangeTag::Equal)
        .count();

    num_shared_lines.try_into().unwrap()
}

/// Returns the percentage of lines from `ref_lines` that also exist in `comp_lines`.
///
/// # Examples
///
/// ```
/// //                ✓   ✓  x   ✓   x      = 3 / 5 = 0.6
/// let ref_lines = "12\n14\n5\n17\n19\n";
/// let comp_lines = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_perc_shared_lines(ref_lines, comp_lines);
/// assert_eq!(result, 0.6);
/// ```
/// ---
/// ```
/// //                ✓   ✓  x   x    = 2 / 4 = 0.5
/// let ref_lines = "12\n14\n5\n17";
/// let comp_lines = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_perc_shared_lines(ref_lines, comp_lines);
/// assert_eq!(result, 0.5);
/// ```
///
pub fn get_perc_shared_lines(ref_lines: &str, comp_lines: &str) -> f32 {
    let num_shared_lines = get_num_shared_lines(ref_lines, comp_lines);

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

        let comp_lines = fs::read_to_string(&path_in_dir).unwrap();
        let perc_shared = get_perc_shared_lines(&ref_lines, &comp_lines);
        path_to_perc_shared.insert(path_in_dir.clone(), perc_shared);
    }

    Ok(path_to_perc_shared)
}
