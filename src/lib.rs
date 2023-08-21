use glob::Pattern;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use similar::{ChangeTag, TextDiff};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use term_grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use walkdir::WalkDir;

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
pub fn get_num_shared_lines(ref_lines: &str, comp_lines: &str) -> usize {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    // let mut num_shared_lines = 0;
    // Increment the number of shared lines for each equal line
    let num_shared_lines = diff
        .iter_all_changes()
        .filter(|change| change.tag() == ChangeTag::Equal)
        .count();

    num_shared_lines
}

#[derive(Debug, PartialEq)]
pub struct Args {
    pub reference_string: String,
    pub search_path: PathBuf,
    pub max_lines: u32,
    pub include_patterns: Option<Vec<Pattern>>,
    pub exclude_patterns: Option<Vec<Pattern>>,
    pub count: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileMatch {
    pub path: PathBuf,
    pub perc_shared: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct FileMatches(pub Vec<FileMatch>);

impl std::ops::Deref for FileMatches {
    type Target = Vec<FileMatch>;
    fn deref(&self) -> &Vec<FileMatch> {
        &self.0
    }
}

impl std::ops::DerefMut for FileMatches {
    fn deref_mut(&mut self) -> &mut Vec<FileMatch> {
        &mut self.0
    }
}

impl FileMatches {
    /// Returns a formatted string with one file match per line with path
    /// string, visualization, and match percentage.
    ///
    /// # Examples
    ///
    /// ```
    /// let file_matches = busca::FileMatches(vec![
    ///     busca::FileMatch {
    ///         path: std::path::PathBuf::from(r"/sample-comprehensive/projects/Geocoding/geocoding.py"),
    ///         perc_shared: 0.9846,
    ///     },
    ///     busca::FileMatch {
    ///         path: std::path::PathBuf::from(
    ///             r"sample-comprehensive\\projects\\Bouncing_ball_simulator\\ball_bounce.py",
    ///         ),
    ///         perc_shared: 0.3481,
    ///     },
    ///     busca::FileMatch {
    ///         path: std::path::PathBuf::from(r"/sample-comprehensive/projects/Geocoding/geocoding.py"),
    ///         perc_shared: 0.0521,
    ///     },
    /// ]);

    /// let expected_output = "\
    /// /sample-comprehensive/projects/Geocoding/geocoding.py                    ++++++++++  98.5%
    /// sample-comprehensive\\\\projects\\\\Bouncing_ball_simulator\\\\ball_bounce.py  +++         34.8%
    /// /sample-comprehensive/projects/Geocoding/geocoding.py                    +            5.2%";

    /// assert_eq!(file_matches.get_formatted_string(), expected_output);
    /// ```
    ///
    pub fn get_formatted_string(&self) -> String {
        let mut grid = Grid::new(GridOptions {
            filling: Filling::Spaces(2),
            direction: Direction::LeftToRight,
        });

        for path_and_perc in self.iter() {
            // Add first column with the file path
            grid.add(Cell::from(path_and_perc.path.display().to_string()));

            // Add second column with the visual indicator of the match perc
            let visual_indicator = "+".repeat((path_and_perc.perc_shared * 10.0).round() as usize);
            let vis_cell = Cell::from(visual_indicator);
            grid.add(vis_cell);

            // Add third column with the numerical match perc
            let perc_str = format!("{:.1}%", (path_and_perc.perc_shared * 100.0));
            let mut perc_cell = Cell::from(perc_str);
            perc_cell.alignment = Alignment::Right;
            grid.add(perc_cell);
        }

        let disp = grid.fit_into_columns(3);

        let mut display_string = disp.to_string();

        // Remove trailing new line
        if display_string.ends_with('\n') {
            display_string.pop();
        }

        display_string
    }
}

impl fmt::Display for FileMatches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let grid_str = self.get_formatted_string();
        write!(f, "{}", grid_str)
    }
}

pub fn run_search(args: &Args) -> Result<FileMatches, String> {
    let walkdir_vec = WalkDir::new(&args.search_path)
        .into_iter()
        .collect::<Vec<_>>();

    let mut file_match_vec: Vec<FileMatch> = walkdir_vec
        .par_iter()
        .filter_map(|dir_entry_result| match dir_entry_result {
            Ok(dir_entry) => compare_file(dir_entry.path(), args, &args.reference_string),
            Err(_) => None,
        })
        .collect();

    // Sort by percent match
    file_match_vec.sort_by(|a, b| {
        b.perc_shared
            .partial_cmp(&a.perc_shared)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Keep the top matches
    file_match_vec.truncate(args.count.into());

    Ok(FileMatches(file_match_vec))
}
#[cfg(test)]
mod test_run_search {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py")
                .unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_lines: 5000,
            include_patterns: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_patterns: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: 2,
        }
    }

    #[test]
    fn normal_search() {
        let valid_args = get_valid_args();

        let expected = FileMatches(vec![
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/nested_dir/ref_B.py"),
                perc_shared: 1.0,
            },
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/file_1.py"),
                perc_shared: 0.14814815,
            },
        ]);
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn include_glob() {
        let mut valid_args = get_valid_args();
        valid_args.include_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = FileMatches(vec![FileMatch {
            path: PathBuf::from("sample_dir_hello_world/nested_dir/sample_json.json"),
            perc_shared: 0.0,
        }]);
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn exclude_glob() {
        let mut valid_args = get_valid_args();
        valid_args.exclude_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = FileMatches(vec![
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/nested_dir/ref_B.py"),
                perc_shared: 1.0,
            },
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/file_1.py"),
                perc_shared: 0.14814815,
            },
        ]);
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }
}

pub fn compare_file(comp_path: &Path, args: &Args, ref_lines: &str) -> Option<FileMatch> {
    // Skip paths that are not files
    if !comp_path.is_file() {
        return None;
    }

    // Skip paths that do not contain the include glob pattern
    match &args.include_patterns {
        Some(include_pattern_vec) => {
            let contains_glob_pattern = include_pattern_vec
                .iter()
                .any(|include_pattern| include_pattern.matches_path(comp_path));

            if !contains_glob_pattern {
                return None;
            }
        }
        None => {}
    };

    // Skip paths that contain the exclude glob pattern
    match &args.exclude_patterns {
        Some(exclude_pattern_vec) => {
            let contains_glob_pattern = exclude_pattern_vec
                .iter()
                .any(|include_pattern| include_pattern.matches_path(comp_path));

            if contains_glob_pattern {
                return None;
            }
        }
        None => (),
    };

    let comp_reader = fs::read_to_string(comp_path);
    let comp_lines = match comp_reader {
        Ok(lines) => lines,
        Err(error) => match error.kind() {
            std::io::ErrorKind::InvalidData => return None,
            other_error => panic!("{:?}", other_error),
        },
    };

    let num_comp_lines = comp_lines.lines().count();
    if (num_comp_lines > args.max_lines as usize) | (num_comp_lines == 0) {
        return None;
    }

    let perc_shared = get_perc_shared_lines(ref_lines, &comp_lines);

    Some(FileMatch {
        path: PathBuf::from(comp_path),
        perc_shared,
    })
}
#[cfg(test)]
mod test_compare_file {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/file_2.py").unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_lines: 5000,
            include_patterns: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_patterns: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: 8,
        }
    }

    #[test]
    fn skip_directory() {
        let valid_args = get_valid_args();

        let ref_lines =
            fs::read_to_string("sample_dir_hello_world/nested_dir/sample_python_file_3.py")
                .unwrap();

        let dir_entry_result = WalkDir::new("sample_dir_hello_world")
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.path(), &valid_args, &ref_lines);

        assert_eq!(file_comparison, None);
    }

    #[test]
    fn same_file_comparison() {
        let valid_args = get_valid_args();

        let file_path_str = "sample_dir_hello_world/nested_dir/sample_python_file_3.py";

        let ref_lines = fs::read_to_string(file_path_str).unwrap();

        let dir_entry_result = WalkDir::new(file_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.path(), &valid_args, &ref_lines);

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(file_path_str),
                perc_shared: 1.0
            })
        );
    }

    #[test]
    fn normal_file_comp() {
        let valid_args = get_valid_args();

        let ref_lines = fs::read_to_string(PathBuf::from(
            "sample_dir_hello_world/nested_dir/sample_python_file_3.py",
        ))
        .unwrap();

        let comp_path_str = "sample_dir_hello_world/file_1.py";

        let dir_entry_result = WalkDir::new(comp_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.path(), &valid_args, &ref_lines);

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(comp_path_str),
                perc_shared: 0.6
            })
        );
    }

    #[test]
    fn include_glob() {
        let mut valid_args = get_valid_args();
        valid_args.include_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let comp_path_str = "sample_dir_hello_world/nested_dir/sample_json.json";

        let dir_entry_result = WalkDir::new(comp_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.path(), &valid_args, "");

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(comp_path_str),
                perc_shared: 0.0
            })
        );
    }
    #[test]
    fn exclude_glob() {
        let mut valid_args = get_valid_args();
        valid_args.exclude_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let comp_path_str = "sample_dir_hello_world/nested_dir/sample_json.json";

        let dir_entry_result = WalkDir::new(comp_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.path(), &valid_args, "");

        assert_eq!(file_comparison, None);
    }
}
