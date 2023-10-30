use glob::Pattern;
use memoize::memoize;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::prelude::IntoParallelIterator;
use similar::TextDiff;
use std::fs::{self};
use std::path::PathBuf;
use term_grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use walkdir::{DirEntry, Error, WalkDir};

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq)]
pub struct FileMatch {
    pub path: PathBuf,
    pub percent_match: f32,
    pub lines: String,
}
#[pymethods]
impl FileMatch {
    #[new]
    fn new(path: PathBuf, percent_match: f32, lines: String) -> Self {
        Self {
            path,
            percent_match,
            lines,
        }
    }
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// Returns a formatted string with one file match per line with path
/// string, visualization, and match percentage.
///
/// # Examples
///
/// ```
/// let file_matches = vec![
///     busca::FileMatch {
///         path: std::path::PathBuf::from("sample-comprehensive/projects/Geocoding/geocoding.py"),
///         percent_match: 0.9846,
///         lines: std::fs::read_to_string("sample-comprehensive/projects/Geocoding/geocoding.py").unwrap(),
///     },
///     busca::FileMatch {
///         path: std::path::PathBuf::from(
///             "sample-comprehensive/projects/Bouncing_ball_simulator/ball_bounce.py"
///         ),
///         percent_match: 0.3481,
///         lines: std::fs::read_to_string(
///             "sample-comprehensive/projects/Bouncing_ball_simulator/ball_bounce.py"
///         ).unwrap(),
///     },
///     busca::FileMatch {
///         path: std::path::PathBuf::from("sample-comprehensive/projects/chatbot/bot.py"),
///         percent_match: 0.0521,
///         lines: std::fs::read_to_string("sample-comprehensive/projects/chatbot/bot.py").unwrap(),
///     },
/// ];

/// let expected_output = "\
/// sample-comprehensive/projects/Geocoding/geocoding.py                  ++++++++++  98.5%
/// sample-comprehensive/projects/Bouncing_ball_simulator/ball_bounce.py  +++         34.8%
/// sample-comprehensive/projects/chatbot/bot.py                          +            5.2%";

/// assert_eq!(busca::format_file_matches(&file_matches), expected_output);
/// ```
///
pub fn format_file_matches(file_matches: &[FileMatch]) -> String {
    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(2),
        direction: Direction::LeftToRight,
    });

    for path_and_perc in file_matches.iter() {
        // Add first column with the file path
        grid.add(Cell::from(path_and_perc.path.display().to_string()));

        // Add second column with the visual indicator of the match perc
        let visual_indicator = "+".repeat((path_and_perc.percent_match * 10.0).round() as usize);
        let vis_cell = Cell::from(visual_indicator);
        grid.add(vis_cell);

        // Add third column with the numerical match perc
        let perc_str = format!("{:.1}%", (path_and_perc.percent_match * 100.0));
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

#[pyfunction]
fn search_for_lines(
    reference_string: String,
    search_path: PathBuf,
    max_lines: Option<usize>,
    count: Option<usize>,
    include_globs: Option<Vec<String>>,
    exclude_globs: Option<Vec<String>>,
) -> PyResult<Vec<FileMatch>> {
    let include_patterns: Option<Vec<Pattern>> = include_globs.map(|include_substring_vec| {
        include_substring_vec
            .iter()
            .map(|include_substring| parse_glob_pattern(include_substring))
            .collect()
    });

    let exclude_patterns: Option<Vec<Pattern>> = exclude_globs.map(|exclude_substring_vec| {
        exclude_substring_vec
            .iter()
            .map(|exclude_substring| parse_glob_pattern(exclude_substring))
            .collect()
    });

    let args = Args {
        reference_string,
        search_path,
        max_lines,
        include_patterns,
        exclude_patterns,
        count,
    };

    let file_matches = match run_search(&args) {
        Ok(file_matches) => file_matches,
        Err(err) => return Err(PyValueError::new_err(err)),
    };

    Ok(file_matches)
}

pub fn parse_glob_pattern(pattern_string: &str) -> Pattern {
    match glob::Pattern::new(pattern_string) {
        Ok(pattern) => pattern,
        Err(e) => panic!("{:?} for '{}'", e, pattern_string),
    }
}

/// A Python module of the Rust `busca` file matching library.
/// https://github.com/noahbaculi/busca
#[pymodule]
#[pyo3(name = "busca_py")]
fn python_module(_py: Python<'_>, module: &PyModule) -> PyResult<()> {
    module.add_class::<FileMatch>()?;
    module.add_function(wrap_pyfunction!(search_for_lines, module)?)?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub struct Args {
    pub reference_string: String,
    pub search_path: PathBuf,
    pub max_lines: Option<usize>,
    pub include_patterns: Option<Vec<Pattern>>,
    pub exclude_patterns: Option<Vec<Pattern>>,
    pub count: Option<usize>,
}

pub fn run_search(args: &Args) -> Result<Vec<FileMatch>, String> {
    let walkdir_vec = WalkDir::new(&args.search_path)
        .into_iter()
        .collect::<Vec<_>>();

    Ok(compare_files(walkdir_vec.into_par_iter(), args))
}
#[cfg(test)]
mod test_run_search {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py")
                .unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_lines: Some(5000),
            include_patterns: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_patterns: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: Some(2),
        }
    }

    #[test]
    fn normal_search() {
        let valid_args = get_valid_args();

        let expected = vec![
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/nested_dir/ref_B.py"),
                percent_match: 1.0,
                lines: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            },
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/file_1.py"),
                percent_match: 2.0 / 9.0,
                lines: fs::read_to_string("sample_dir_hello_world/file_1.py").unwrap(),
            },
        ];
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn include_glob() {
        let mut valid_args = get_valid_args();
        valid_args.include_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = vec![FileMatch {
            path: PathBuf::from("sample_dir_hello_world/nested_dir/sample_json.json"),
            percent_match: 0.0,
            lines: fs::read_to_string("sample_dir_hello_world/nested_dir/sample_json.json")
                .unwrap(),
        }];
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn exclude_glob() {
        let mut valid_args = get_valid_args();
        valid_args.exclude_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = vec![
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/nested_dir/ref_B.py"),
                percent_match: 1.0,
                lines: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            },
            FileMatch {
                path: PathBuf::from("sample_dir_hello_world/file_1.py"),
                percent_match: 2.0 / 9.0,
                lines: fs::read_to_string("sample_dir_hello_world/file_1.py").unwrap(),
            },
        ];
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }
}

pub fn compare_files(
    walkdir_iterator: impl ParallelIterator<Item = Result<DirEntry, Error>>,
    args: &Args,
) -> Vec<FileMatch> {
    let mut file_match_vec: Vec<FileMatch> = walkdir_iterator
        .filter_map(|dir_entry_result| match dir_entry_result {
            Ok(dir_entry) => compare_file(dir_entry.into_path(), args, &args.reference_string),
            Err(_) => None,
        })
        .collect();

    // Sort by percent match
    file_match_vec.sort_by(|a, b| {
        b.percent_match
            .partial_cmp(&a.percent_match)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(count) = args.count {
        // Keep the top matches
        file_match_vec.truncate(count);
    }

    file_match_vec
}

pub fn compare_file(comp_path: PathBuf, args: &Args, ref_lines: &str) -> Option<FileMatch> {
    // Skip paths that are not files
    if !comp_path.is_file() {
        return None;
    }

    // Skip paths that do not contain the include glob pattern
    match &args.include_patterns {
        Some(include_pattern_vec) => {
            let contains_glob_pattern = include_pattern_vec
                .par_iter()
                .any(|include_pattern| include_pattern.matches_path(comp_path.as_path()));

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
                .par_iter()
                .any(|include_pattern| include_pattern.matches_path(comp_path.as_path()));

            if contains_glob_pattern {
                return None;
            }
        }
        None => (),
    };

    let comp_lines = match read_file(comp_path.clone()) {
        Some(value) => value,
        None => return None,
    };

    if let Some(max_lines) = args.max_lines {
        let num_comp_lines = comp_lines.lines().count();
        if (num_comp_lines > max_lines) | (num_comp_lines == 0) {
            return None;
        }
    }

    let percent_match = get_percent_matching_lines(ref_lines, &comp_lines);

    Some(FileMatch {
        path: comp_path,
        percent_match,
        lines: comp_lines,
    })
}

#[memoize]
fn read_file(comp_path: PathBuf) -> Option<String> {
    let comp_reader = fs::read_to_string(comp_path);
    let comp_lines = match comp_reader {
        Ok(lines) => lines,
        Err(error) => match error.kind() {
            std::io::ErrorKind::InvalidData => return None,
            other_error => panic!("{:?}", other_error),
        },
    };
    Some(comp_lines)
}

// pub fn flush_read_file_cache() {
//     memoized_flush_read_file()
// }

/// Returns the percentage of lines from `ref_lines` that also exist in `comp_lines`.
///
///
/// # Examples
///
/// ```
/// //                ✓   ✓  x   ✓   x      = 3
/// let ref_lines = "12\n14\n5\n17\n19\n";
/// let comp_lines = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_percent_matching_lines(ref_lines, comp_lines);
/// assert_eq!(result, 3.0 / 7.0);
/// ```
/// ---
/// ```
/// //                ✓   ✓  x   x    = 2 / 4 = 0.5
/// let ref_lines = "12\n14\n5\n17";
/// let comp_lines = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_percent_matching_lines(ref_lines, comp_lines);
/// assert_eq!(result, 4.0 / 13.0);
/// ```
///
pub fn get_percent_matching_lines(ref_lines: &str, comp_lines: &str) -> f32 {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);
    diff.ratio()
}

#[cfg(test)]
mod test_compare_file {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/file_2.py").unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_lines: Some(5000),
            include_patterns: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_patterns: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: Some(8),
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

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, &ref_lines);

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

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, &ref_lines);

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(file_path_str),
                percent_match: 1.0,
                lines: ref_lines,
            })
        );
    }

    #[test]
    fn normal_file_comp() {
        let valid_args = get_valid_args();

        let ref_lines =
            fs::read_to_string("sample_dir_hello_world/nested_dir/sample_python_file_3.py")
                .unwrap();

        let comp_path_str = "sample_dir_hello_world/file_1.py";

        let dir_entry_result = WalkDir::new(comp_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, &ref_lines);

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(comp_path_str),
                percent_match: 3.0 / 7.0,
                lines: fs::read_to_string(comp_path_str).unwrap(),
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

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, "");

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(comp_path_str),
                percent_match: 0.0,
                lines: fs::read_to_string(comp_path_str).unwrap(),
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

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, "");

        assert_eq!(file_comparison, None);
    }
}
