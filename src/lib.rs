use glob::Pattern;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::prelude::IntoParallelIterator;
use similar::TextDiff;
use std::fs::{self};
use std::path::PathBuf;
use term_grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use walkdir::WalkDir;

use std::fmt;

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    InvalidGlob {
        pattern: String,
        source: glob::PatternError,
    },
    SearchPathNotFound(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidGlob { pattern, source } => {
                write!(f, "invalid glob '{pattern}': {source}")
            }
            Error::SearchPathNotFound(path) => {
                write!(f, "search path not found: {}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidGlob { source, .. } => Some(source),
            Error::SearchPathNotFound(_) => None,
        }
    }
}

#[pyclass(get_all)]
#[derive(Debug, Clone, PartialEq)]
pub struct FileComparison {
    pub path: PathBuf,
    pub similarity_ratio: f32,
    pub content: String,
}
#[pymethods]
impl FileComparison {
    #[new]
    fn new(path: PathBuf, similarity_ratio: f32, content: String) -> Self {
        Self {
            path,
            similarity_ratio,
            content,
        }
    }
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// Returns a formatted string with one comparison per line: path, a bar
/// visualization of the similarity ratio, and the ratio as a percentage.
///
/// # Examples
///
/// ```
/// let file_comparisons = vec![
///     busca::FileComparison {
///         path: std::path::PathBuf::from(
///             "sample_dir_hello_world/nested_dir/sample_python_file_3.py",
///         ),
///         similarity_ratio: 0.9846,
///         content: std::fs::read_to_string(
///             "sample_dir_hello_world/nested_dir/sample_python_file_3.py",
///         )
///         .unwrap(),
///     },
///     busca::FileComparison {
///         path: std::path::PathBuf::from("sample_dir_hello_world/file_1.py"),
///         similarity_ratio: 0.3481,
///         content: std::fs::read_to_string("sample_dir_hello_world/file_1.py").unwrap(),
///     },
///     busca::FileComparison {
///         path: std::path::PathBuf::from("sample_dir_mix/file_5.py"),
///         similarity_ratio: 0.0521,
///         content: std::fs::read_to_string("sample_dir_mix/file_5.py").unwrap(),
///     },
/// ];
///
/// let expected_output = "\
/// sample_dir_hello_world/nested_dir/sample_python_file_3.py  ++++++++++  98.5%
/// sample_dir_hello_world/file_1.py                           +++         34.8%
/// sample_dir_mix/file_5.py                                   +            5.2%";
///
/// assert_eq!(busca::format_file_comparisons(&file_comparisons), expected_output);
/// ```
///
pub fn format_file_comparisons(file_comparisons: &[FileComparison]) -> String {
    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(2),
        direction: Direction::LeftToRight,
    });

    for file_comparison in file_comparisons.iter() {
        // Add first column with the file path
        grid.add(Cell::from(file_comparison.path.display().to_string()));

        // Add second column with the visual indicator of the similarity ratio
        let visual_indicator = "+".repeat((file_comparison.similarity_ratio * 10.0).round() as usize);
        let vis_cell = Cell::from(visual_indicator);
        grid.add(vis_cell);

        // Add third column with the numerical similarity ratio
        let perc_str = format!("{:.1}%", (file_comparison.similarity_ratio * 100.0));
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

/// A Python module of the Rust `busca` file matching library.
/// https://github.com/noahbaculi/busca
#[pymodule]
mod busca_py {
    use super::*;

    #[pymodule_export]
    use super::FileComparison;

    #[pyfunction]
    #[pyo3(signature = (
        reference_string,
        search_path,
        max_file_lines=None,
        count=None,
        include_glob=None,
        exclude_glob=None
    ))]
    fn search(
        reference_string: String,
        search_path: PathBuf,
        max_file_lines: Option<usize>,
        count: Option<usize>,
        include_glob: Option<Vec<String>>,
        exclude_glob: Option<Vec<String>>,
    ) -> PyResult<Vec<FileComparison>> {
        let args = Args::new(
            reference_string,
            search_path,
            max_file_lines,
            count,
            include_glob.unwrap_or_default(),
            exclude_glob.unwrap_or_default(),
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

        run_search(&args).map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub struct Args {
    pub reference_string: String,
    pub search_path: PathBuf,
    pub max_file_lines: Option<usize>,
    pub include_glob: Option<Vec<Pattern>>,
    pub exclude_glob: Option<Vec<Pattern>>,
    pub count: Option<usize>,
}

impl Args {
    pub fn new(
        reference_string: String,
        search_path: PathBuf,
        max_file_lines: Option<usize>,
        count: Option<usize>,
        include_glob: Vec<String>,
        exclude_glob: Vec<String>,
    ) -> Result<Self, Error> {
        if !search_path.is_file() && !search_path.is_dir() {
            return Err(Error::SearchPathNotFound(search_path));
        }
        let include_glob = parse_glob_vec(include_glob)?;
        let exclude_glob = parse_glob_vec(exclude_glob)?;
        Ok(Self {
            reference_string,
            search_path,
            max_file_lines,
            include_glob,
            exclude_glob,
            count,
        })
    }
}

fn parse_glob_vec(globs: Vec<String>) -> Result<Option<Vec<Pattern>>, Error> {
    if globs.is_empty() {
        return Ok(None);
    }
    globs
        .into_iter()
        .map(|raw| {
            Pattern::new(&raw).map_err(|source| Error::InvalidGlob {
                pattern: raw,
                source,
            })
        })
        .collect::<Result<Vec<Pattern>, _>>()
        .map(Some)
}

pub fn run_search(args: &Args) -> Result<Vec<FileComparison>, Error> {
    run_search_with_progress(args, |_, _| {})
}

pub fn run_search_with_progress<F>(args: &Args, on_progress: F) -> Result<Vec<FileComparison>, Error>
where
    F: Fn(u64, u64) + Send + Sync,
{
    use std::sync::atomic::{AtomicU64, Ordering};

    let dir_entries = WalkDir::new(&args.search_path)
        .into_iter()
        .collect::<Vec<_>>();
    let total = dir_entries.len() as u64;
    let done = AtomicU64::new(0);

    let mut file_comparisons: Vec<FileComparison> = dir_entries
        .into_par_iter()
        .filter_map(|dir_entry_result| {
            let out = match dir_entry_result {
                Ok(dir_entry) => compare_file(dir_entry.into_path(), args, &args.reference_string),
                Err(_) => None,
            };
            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
            on_progress(d, total);
            out
        })
        .collect();

    file_comparisons.sort_by(|a, b| {
        b.similarity_ratio
            .partial_cmp(&a.similarity_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(count) = args.count {
        file_comparisons.truncate(count);
    }

    Ok(file_comparisons)
}
#[cfg(test)]
mod test_run_search {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py")
                .unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_file_lines: Some(5000),
            include_glob: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_glob: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: Some(2),
        }
    }

    #[test]
    fn normal_search() {
        let valid_args = get_valid_args();

        let expected = vec![
            FileComparison {
                path: PathBuf::from("sample_dir_hello_world/nested_dir/ref_B.py"),
                similarity_ratio: 1.0,
                content: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            },
            FileComparison {
                path: PathBuf::from("sample_dir_hello_world/file_1.py"),
                similarity_ratio: 2.0 / 9.0,
                content: fs::read_to_string("sample_dir_hello_world/file_1.py").unwrap(),
            },
        ];
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn include_glob() {
        let mut valid_args = get_valid_args();
        valid_args.include_glob = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = vec![FileComparison {
            path: PathBuf::from("sample_dir_hello_world/nested_dir/sample_json.json"),
            similarity_ratio: 0.0,
            content: fs::read_to_string("sample_dir_hello_world/nested_dir/sample_json.json")
                .unwrap(),
        }];
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn exclude_glob() {
        let mut valid_args = get_valid_args();
        valid_args.exclude_glob = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = vec![
            FileComparison {
                path: PathBuf::from("sample_dir_hello_world/nested_dir/ref_B.py"),
                similarity_ratio: 1.0,
                content: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            },
            FileComparison {
                path: PathBuf::from("sample_dir_hello_world/file_1.py"),
                similarity_ratio: 2.0 / 9.0,
                content: fs::read_to_string("sample_dir_hello_world/file_1.py").unwrap(),
            },
        ];
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }
}

pub fn compare_file(
    candidate_path: PathBuf,
    args: &Args,
    reference_string: &str,
) -> Option<FileComparison> {
    // Skip paths that are not files
    if !candidate_path.is_file() {
        return None;
    }

    // Skip paths that do not match any include glob
    if let Some(include_glob) = &args.include_glob {
        let matches_any_include = include_glob
            .par_iter()
            .any(|glob| glob.matches_path(candidate_path.as_path()));

        if !matches_any_include {
            return None;
        }
    };

    // Skip paths that match any exclude glob
    if let Some(exclude_glob) = &args.exclude_glob {
        let matches_any_exclude = exclude_glob
            .par_iter()
            .any(|glob| glob.matches_path(candidate_path.as_path()));

        if matches_any_exclude {
            return None;
        }
    };

    let candidate_content = read_file(candidate_path.clone())?;

    if let Some(max_file_lines) = args.max_file_lines {
        let num_candidate_lines = candidate_content.lines().count();
        if (num_candidate_lines > max_file_lines) | (num_candidate_lines == 0) {
            return None;
        }
    }

    let similarity_ratio = get_similarity_ratio(reference_string, &candidate_content);

    Some(FileComparison {
        path: candidate_path,
        similarity_ratio,
        content: candidate_content,
    })
}

fn read_file(candidate_path: PathBuf) -> Option<String> {
    let candidate_reader = fs::read_to_string(candidate_path);
    let candidate_content = match candidate_reader {
        Ok(content) => content,
        Err(error) => match error.kind() {
            std::io::ErrorKind::InvalidData => return None,
            other_error => panic!("{:?}", other_error),
        },
    };
    Some(candidate_content)
}

/// Returns the `similar::TextDiff::ratio()` between the reference and candidate
/// strings. This is a Ratcliff/Obershelp similarity over the line sequences,
/// not the fraction of reference lines that appear in the candidate. See
/// ADR-0001 for the rationale.
///
/// # Examples
///
/// ```
/// let reference_string = "12\n14\n5\n17\n19\n";
/// let candidate_content = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_similarity_ratio(reference_string, candidate_content);
/// assert_eq!(result, 3.0 / 7.0);
/// ```
/// ---
/// ```
/// let reference_string = "12\n14\n5\n17";
/// let candidate_content = "11\n12\n13\n14\n15\n16\n\n17\n18\n";
/// let result = busca::get_similarity_ratio(reference_string, candidate_content);
/// assert_eq!(result, 4.0 / 13.0);
/// ```
///
pub fn get_similarity_ratio(reference_string: &str, candidate_content: &str) -> f32 {
    let diff = TextDiff::from_lines(reference_string, candidate_content);
    diff.ratio()
}

#[cfg(test)]
mod test_compare_file {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/file_2.py").unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_file_lines: Some(5000),
            include_glob: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_glob: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: Some(8),
        }
    }

    #[test]
    fn skip_directory() {
        let valid_args = get_valid_args();

        let reference_string =
            fs::read_to_string("sample_dir_hello_world/nested_dir/sample_python_file_3.py")
                .unwrap();

        let dir_entry_result = WalkDir::new("sample_dir_hello_world")
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison =
            compare_file(dir_entry_result.into_path(), &valid_args, &reference_string);

        assert_eq!(file_comparison, None);
    }

    #[test]
    fn same_file_comparison() {
        let valid_args = get_valid_args();

        let file_path_str = "sample_dir_hello_world/nested_dir/sample_python_file_3.py";

        let reference_string = fs::read_to_string(file_path_str).unwrap();

        let dir_entry_result = WalkDir::new(file_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison =
            compare_file(dir_entry_result.into_path(), &valid_args, &reference_string);

        assert_eq!(
            file_comparison,
            Some(FileComparison {
                path: PathBuf::from(file_path_str),
                similarity_ratio: 1.0,
                content: reference_string,
            })
        );
    }

    #[test]
    fn normal_file_comparison() {
        let valid_args = get_valid_args();

        let reference_string =
            fs::read_to_string("sample_dir_hello_world/nested_dir/sample_python_file_3.py")
                .unwrap();

        let candidate_path_str = "sample_dir_hello_world/file_1.py";

        let dir_entry_result = WalkDir::new(candidate_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison =
            compare_file(dir_entry_result.into_path(), &valid_args, &reference_string);

        assert_eq!(
            file_comparison,
            Some(FileComparison {
                path: PathBuf::from(candidate_path_str),
                similarity_ratio: 3.0 / 7.0,
                content: fs::read_to_string(candidate_path_str).unwrap(),
            })
        );
    }

    #[test]
    fn include_glob() {
        let mut valid_args = get_valid_args();
        valid_args.include_glob = Some(vec![Pattern::new("*.json").unwrap()]);

        let candidate_path_str = "sample_dir_hello_world/nested_dir/sample_json.json";

        let dir_entry_result = WalkDir::new(candidate_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, "");

        assert_eq!(
            file_comparison,
            Some(FileComparison {
                path: PathBuf::from(candidate_path_str),
                similarity_ratio: 0.0,
                content: fs::read_to_string(candidate_path_str).unwrap(),
            })
        );
    }

    #[test]
    fn exclude_glob() {
        let mut valid_args = get_valid_args();
        valid_args.exclude_glob = Some(vec![Pattern::new("*.json").unwrap()]);

        let candidate_path_str = "sample_dir_hello_world/nested_dir/sample_json.json";

        let dir_entry_result = WalkDir::new(candidate_path_str)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_comparison = compare_file(dir_entry_result.into_path(), &valid_args, "");

        assert_eq!(file_comparison, None);
    }
}

#[cfg(test)]
mod test_error {
    use super::Error;
    use std::path::PathBuf;

    #[test]
    fn invalid_glob_display() {
        let err = Error::InvalidGlob {
            pattern: "[".into(),
            source: glob::Pattern::new("[").unwrap_err(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid glob"), "got: {msg}");
        assert!(msg.contains("["), "got: {msg}");
    }

    #[test]
    fn search_path_not_found_display() {
        let err = Error::SearchPathNotFound(PathBuf::from("/nope"));
        assert_eq!(err.to_string(), "search path not found: /nope");
    }

    #[test]
    fn invalid_glob_has_source() {
        let err = Error::InvalidGlob {
            pattern: "[".into(),
            source: glob::Pattern::new("[").unwrap_err(),
        };
        assert!(std::error::Error::source(&err).is_some());
    }
}

#[cfg(test)]
mod test_run_search_with_progress {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn callback_invoked_during_search() {
        let args = Args::new(
            fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            Some(2),
            vec!["*.py".into()],
            vec!["*.yml".into()],
        )
        .unwrap();

        let counter = AtomicU64::new(0);
        let result = run_search_with_progress(&args, |_done, _total| {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

        assert!(!result.is_empty());
        assert!(counter.load(Ordering::SeqCst) > 0);
    }
}

#[cfg(test)]
mod test_args_new {
    use super::*;

    #[test]
    fn happy_path() {
        let args = Args::new(
            "ref".into(),
            PathBuf::from("sample_dir_hello_world"),
            Some(1000),
            Some(5),
            vec!["*.py".into()],
            vec!["*.yml".into()],
        )
        .unwrap();
        assert_eq!(args.max_file_lines, Some(1000));
        assert_eq!(args.count, Some(5));
        assert!(args.include_glob.is_some());
        assert!(args.exclude_glob.is_some());
    }

    #[test]
    fn empty_glob_vec_is_no_filter() {
        let args = Args::new(
            "ref".into(),
            PathBuf::from("sample_dir_hello_world"),
            None,
            None,
            vec![],
            vec![],
        )
        .unwrap();
        assert!(args.include_glob.is_none());
        assert!(args.exclude_glob.is_none());
    }

    #[test]
    fn invalid_include_glob_errors() {
        let result = Args::new(
            "ref".into(),
            PathBuf::from("sample_dir_hello_world"),
            None,
            None,
            vec!["[".into()],
            vec![],
        );
        match result {
            Err(Error::InvalidGlob { pattern, .. }) => assert_eq!(pattern, "["),
            other => panic!("expected InvalidGlob, got {:?}", other),
        }
    }

    #[test]
    fn invalid_exclude_glob_errors() {
        let result = Args::new(
            "ref".into(),
            PathBuf::from("sample_dir_hello_world"),
            None,
            None,
            vec![],
            vec!["[".into()],
        );
        assert!(matches!(result, Err(Error::InvalidGlob { .. })));
    }

    #[test]
    fn missing_search_path_errors() {
        let bogus = PathBuf::from("/definitely/not/a/real/path/xyz123");
        let result = Args::new(
            "ref".into(),
            bogus.clone(),
            None,
            None,
            vec![],
            vec![],
        );
        match result {
            Err(Error::SearchPathNotFound(p)) => assert_eq!(p, bogus),
            other => panic!("expected SearchPathNotFound, got {:?}", other),
        }
    }
}
