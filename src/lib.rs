#![warn(clippy::perf, clippy::complexity)]
use glob::Pattern;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::prelude::IntoParallelIterator;
use similar::{DiffableStr, TextDiff};
use std::collections::{BinaryHeap, HashMap};
use std::fs::{self};
use std::path::{Path, PathBuf};
use term_grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
use walkdir::{DirEntry, WalkDir};

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
///         content: "print(\"Hello World\")\n".to_string(),
///     },
///     busca::FileComparison {
///         path: std::path::PathBuf::from("sample_dir_hello_world/file_1.py"),
///         similarity_ratio: 0.3481,
///         content: "print(\"Hello\")\n".to_string(),
///     },
///     busca::FileComparison {
///         path: std::path::PathBuf::from("sample_dir_mix/file_5.py"),
///         similarity_ratio: 0.0521,
///         content: String::new(),
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
        let visual_indicator =
            "+".repeat((file_comparison.similarity_ratio * 10.0).round() as usize);
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

    // Remove the trailing newline
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
        include_glob: Option<Bound<'_, PyAny>>,
        exclude_glob: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Vec<FileComparison>> {
        let include_glob = extract_glob_arg(include_glob)?;
        let exclude_glob = extract_glob_arg(exclude_glob)?;

        let args = Args::new(
            reference_string,
            search_path,
            max_file_lines,
            count,
            include_glob,
            exclude_glob,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

        run_search(&args).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn extract_glob_arg(obj: Option<Bound<'_, PyAny>>) -> PyResult<Vec<String>> {
        let Some(obj) = obj else {
            return Ok(Vec::new());
        };
        if obj.is_none() {
            return Ok(Vec::new());
        }
        if let Ok(s) = obj.extract::<String>() {
            return Ok(vec![s]);
        }
        obj.extract::<Vec<String>>().map_err(|e| {
            PyValueError::new_err(format!(
                "glob argument must be a string, a list of strings, or None: {e}"
            ))
        })
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub struct Args {
    pub reference_string: String,
    pub search_path: PathBuf,
    pub max_file_lines: Option<usize>,
    pub(crate) include_glob: Option<Vec<Pattern>>,
    pub(crate) exclude_glob: Option<Vec<Pattern>>,
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

pub fn run_search_with_progress<F>(
    args: &Args,
    on_progress: F,
) -> Result<Vec<FileComparison>, Error>
where
    F: Fn(u64, u64) + Send + Sync,
{
    use std::sync::atomic::{AtomicU64, Ordering};

    let dir_entries = WalkDir::new(&args.search_path)
        .into_iter()
        .collect::<Vec<_>>();
    let total = dir_entries.len() as u64;
    let done = AtomicU64::new(0);

    match args.count {
        None => {
            let mut file_comparisons: Vec<FileComparison> = dir_entries
                .into_par_iter()
                .filter_map(|dir_entry_result| {
                    let out = match dir_entry_result {
                        Ok(dir_entry) => compare_file(dir_entry, args, &args.reference_string),
                        Err(_) => None,
                    };
                    let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                    on_progress(d, total);
                    out
                })
                .collect();

            file_comparisons.sort_by(|a, b| {
                b.similarity_ratio
                    .partial_cmp(&a.similarity_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            Ok(file_comparisons)
        }
        Some(count) => {
            let reference_index = ReferenceIndex::new(&args.reference_string);
            let collected = dir_entries
                .into_par_iter()
                .enumerate()
                .fold(
                    || TopN::new(count),
                    |mut heap, (walk_index, dir_entry_result)| {
                        if let Ok(dir_entry) = dir_entry_result {
                            if let Some(comparison) =
                                score_candidate_bounded(dir_entry, args, &reference_index, &heap)
                            {
                                heap.push(walk_index, comparison);
                            }
                        }
                        let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                        on_progress(d, total);
                        heap
                    },
                )
                .reduce(|| TopN::new(count), TopN::merge);

            Ok(collected.into_sorted_vec())
        }
    }
}

/// Scores one candidate for the bounded search. Skips the full `TextDiff::ratio`
/// whenever a cheap upper bound proves the file cannot enter the current top-N.
fn score_candidate_bounded(
    dir_entry: DirEntry,
    args: &Args,
    reference: &ReferenceIndex,
    heap: &TopN,
) -> Option<FileComparison> {
    let (candidate_path, candidate_content) = read_candidate(dir_entry, args)?;

    // max_file_lines uses str::lines().count(), identical to the unbounded path,
    // so behavior is preserved even for files with lone carriage returns (where
    // similar's tokenize_lines would count differently). This is why the double
    // line scan is not merged away here: the two counts use deliberately
    // different tokenizers.
    if let Some(max_file_lines) = args.max_file_lines {
        let num_candidate_lines = candidate_content.lines().count();
        if (num_candidate_lines > max_file_lines) | (num_candidate_lines == 0) {
            return None;
        }
    }

    // The multiset and token count for the upper bounds use similar's tokenizer.
    let (cand_counts, cand_len) = line_counts(&candidate_content);

    if !heap.should_compute(real_quick_ratio(reference.len, cand_len)) {
        return None;
    }

    let quick = quick_ratio_bound(&reference.counts, reference.len, &cand_counts, cand_len);
    if !heap.should_compute(quick) {
        return None;
    }

    let similarity_ratio = get_similarity_ratio(&args.reference_string, &candidate_content);

    Some(FileComparison {
        path: candidate_path,
        similarity_ratio,
        content: candidate_content,
    })
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

    fn args_with_count(count: Option<usize>) -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py")
                .unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_file_lines: Some(5000),
            include_glob: None,
            exclude_glob: None,
            count,
        }
    }

    #[test]
    fn bounded_matches_unbounded_then_truncate() {
        // The unbounded path is the reference implementation: compute every
        // candidate, stable-sort, then truncate. The bounded path must return a
        // byte-identical prefix for every count, including ties and 0.0 fills.
        let mut reference = run_search(&args_with_count(None)).unwrap();
        for n in [0usize, 1, 2, 3, 100] {
            let mut expected = reference.clone();
            expected.truncate(n);
            let bounded = run_search(&args_with_count(Some(n))).unwrap();
            assert_eq!(bounded, expected, "mismatch at count {n}");
        }
        // Guard against accidental mutation of `reference` above.
        reference.truncate(0);
        assert!(reference.is_empty());
    }

    #[test]
    fn bounded_matches_unbounded_with_globs() {
        let with_globs = |count| Args {
            include_glob: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_glob: Some(vec![Pattern::new("*.json").unwrap()]),
            ..args_with_count(count)
        };
        let mut reference = run_search(&with_globs(None)).unwrap();
        for n in [1usize, 2, 50] {
            let mut expected = reference.clone();
            expected.truncate(n);
            let bounded = run_search(&with_globs(Some(n))).unwrap();
            assert_eq!(bounded, expected, "glob mismatch at count {n}");
        }
        reference.truncate(0);
    }
}

/// Applies the file-type and glob filters and reads the candidate's content.
/// Returns `None` when the entry is filtered out or cannot be read as UTF-8.
fn read_candidate(dir_entry: DirEntry, args: &Args) -> Option<(PathBuf, String)> {
    // file_type() comes from the directory read at no extra syscall, where
    // is_file() would re-stat every candidate. A symlink reports as neither file
    // nor directory, so follow it with is_file() to match the old behavior.
    let file_type = dir_entry.file_type();
    let is_file = file_type.is_file() || (file_type.is_symlink() && dir_entry.path().is_file());
    if !is_file {
        return None;
    }

    let candidate_path = dir_entry.into_path();

    if let Some(include_glob) = &args.include_glob {
        let matches_any_include = include_glob
            .iter()
            .any(|glob| glob.matches_path(candidate_path.as_path()));
        if !matches_any_include {
            return None;
        }
    }

    if let Some(exclude_glob) = &args.exclude_glob {
        let matches_any_exclude = exclude_glob
            .iter()
            .any(|glob| glob.matches_path(candidate_path.as_path()));
        if matches_any_exclude {
            return None;
        }
    }

    let candidate_content = read_file(&candidate_path)?;
    Some((candidate_path, candidate_content))
}

pub(crate) fn compare_file(
    dir_entry: DirEntry,
    args: &Args,
    reference_string: &str,
) -> Option<FileComparison> {
    let (candidate_path, candidate_content) = read_candidate(dir_entry, args)?;

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

fn read_file(candidate_path: &Path) -> Option<String> {
    match fs::read_to_string(candidate_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => None,
        Err(error) => {
            eprintln!("busca: skipping {}: {}", candidate_path.display(), error);
            None
        }
    }
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

/// Builds a line-count multiset using `similar`'s own line tokenizer, so the
/// counts share the exact tokenization (and denominator) `TextDiff::from_lines`
/// uses. Returns each distinct line's count and the total token count.
fn line_counts(s: &str) -> (HashMap<&str, u32>, usize) {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    let mut len = 0;
    for token in s.tokenize_lines() {
        *counts.entry(token).or_insert(0) += 1;
        len += 1;
    }
    (counts, len)
}

/// The reference file's line multiset and token count, built once and shared
/// across every candidate comparison.
struct ReferenceIndex<'a> {
    counts: HashMap<&'a str, u32>,
    len: usize,
}

impl<'a> ReferenceIndex<'a> {
    fn new(reference: &'a str) -> Self {
        let (counts, len) = line_counts(reference);
        Self { counts, len }
    }
}

/// Length-only upper bound on `similar`'s ratio: matches cannot exceed the
/// shorter token sequence. Nearly free, used as a pre-filter.
fn real_quick_ratio(ref_len: usize, cand_len: usize) -> f32 {
    let total = ref_len + cand_len;
    if total == 0 {
        return 1.0;
    }
    2.0 * ref_len.min(cand_len) as f32 / total as f32
}

/// Line-multiset upper bound on `similar`'s ratio: matches cannot exceed the
/// multiset intersection of the two line sequences. Tighter than
/// `real_quick_ratio` and always less than or equal to it.
fn quick_ratio_bound(
    ref_counts: &HashMap<&str, u32>,
    ref_len: usize,
    cand_counts: &HashMap<&str, u32>,
    cand_len: usize,
) -> f32 {
    let total = ref_len + cand_len;
    if total == 0 {
        return 1.0;
    }
    // Iterate the smaller map; the intersection sum is symmetric.
    let (small, large) = if cand_counts.len() <= ref_counts.len() {
        (cand_counts, ref_counts)
    } else {
        (ref_counts, cand_counts)
    };
    let mut matches = 0u32;
    for (token, &count) in small {
        if let Some(&other) = large.get(token) {
            matches += count.min(other);
        }
    }
    2.0 * matches as f32 / total as f32
}

/// One result inside a `TopN` heap. Ordered so the "greatest" entry is the one
/// to evict first: lowest ratio, and for ties the highest walk index.
struct HeapEntry {
    walk_index: usize,
    comparison: FileComparison,
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.walk_index == other.walk_index
            && self.comparison.similarity_ratio == other.comparison.similarity_ratio
    }
}
impl Eq for HeapEntry {}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Greater means more evictable: lower ratio first, then higher index.
        other
            .comparison
            .similarity_ratio
            .partial_cmp(&self.comparison.similarity_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(self.walk_index.cmp(&other.walk_index))
    }
}

/// A bounded collector that retains at most `capacity` highest-ratio results.
/// Backed by a max-heap whose top is the next entry to evict.
struct TopN {
    capacity: usize,
    heap: BinaryHeap<HeapEntry>,
}

impl TopN {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            heap: BinaryHeap::new(),
        }
    }

    /// Whether a candidate with this upper bound could still place. Computing the
    /// full diff is only worthwhile when the heap is not yet full, or the bound
    /// is at least the lowest kept ratio. Equality computes, so ties at the
    /// threshold are resolved by the true ratio and walk index, not pruned.
    fn should_compute(&self, upper_bound: f32) -> bool {
        if self.capacity == 0 {
            return false;
        }
        if self.heap.len() < self.capacity {
            return true;
        }
        match self.heap.peek() {
            Some(worst) => upper_bound >= worst.comparison.similarity_ratio,
            None => true,
        }
    }

    fn push(&mut self, walk_index: usize, comparison: FileComparison) {
        self.push_entry(HeapEntry {
            walk_index,
            comparison,
        });
    }

    fn push_entry(&mut self, entry: HeapEntry) {
        if self.capacity == 0 {
            return;
        }
        self.heap.push(entry);
        if self.heap.len() > self.capacity {
            self.heap.pop();
        }
    }

    fn merge(mut self, other: TopN) -> TopN {
        for entry in other.heap {
            self.push_entry(entry);
        }
        self
    }

    fn into_sorted_vec(self) -> Vec<FileComparison> {
        let mut entries = self.heap.into_vec();
        entries.sort_by(|a, b| {
            b.comparison
                .similarity_ratio
                .partial_cmp(&a.comparison.similarity_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.walk_index.cmp(&b.walk_index))
        });
        entries.into_iter().map(|e| e.comparison).collect()
    }
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

        let file_comparison = compare_file(dir_entry_result, &valid_args, &reference_string);

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

        let file_comparison = compare_file(dir_entry_result, &valid_args, &reference_string);

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

        let file_comparison = compare_file(dir_entry_result, &valid_args, &reference_string);

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

        let file_comparison = compare_file(dir_entry_result, &valid_args, "");

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

        let file_comparison = compare_file(dir_entry_result, &valid_args, "");

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
        let result = Args::new("ref".into(), bogus.clone(), None, None, vec![], vec![]);
        match result {
            Err(Error::SearchPathNotFound(p)) => assert_eq!(p, bogus),
            other => panic!("expected SearchPathNotFound, got {:?}", other),
        }
    }
}

#[cfg(test)]
mod test_read_file {
    use super::*;

    #[test]
    fn returns_none_on_invalid_data() {
        let path = PathBuf::from("sample_dir_hello_world/nested_dir/sample_json.json");
        let result = read_file(&path);
        assert!(result.is_some(), "json file should read as UTF-8");
    }

    #[test]
    fn returns_none_on_directory_read_without_panicking() {
        let path = PathBuf::from("sample_dir_hello_world");
        let result = read_file(&path);
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod test_line_tokens {
    use super::*;

    #[test]
    fn line_counts_match_textdiff_tokenization() {
        // `from_lines(s, "").old_slices()` is exactly how `similar` tokenizes the
        // old side, so `line_counts` must agree on count and on every token.
        for s in [
            "a\nb\nc\n",
            "a\nb\nc",
            "a\n\nb\n",
            "single line no newline",
            "trailing\n",
            "carriage\r\nreturn\r\n",
            "lone\rcarriage",
        ] {
            let (counts, len) = line_counts(s);
            let expected: Vec<&str> = TextDiff::from_lines(s, "").old_slices().to_vec();
            assert_eq!(len, expected.len(), "token count mismatch for {s:?}");
            let total: u32 = counts.values().sum();
            assert_eq!(
                total as usize,
                expected.len(),
                "multiset total mismatch for {s:?}"
            );
            for token in &expected {
                assert!(
                    counts.contains_key(token),
                    "missing token {token:?} for {s:?}"
                );
            }
        }
    }

    #[test]
    fn line_counts_multiset_is_correct() {
        let (counts, len) = line_counts("a\na\nb\n");
        assert_eq!(len, 3);
        assert_eq!(counts.get("a\n"), Some(&2));
        assert_eq!(counts.get("b\n"), Some(&1));
    }
}

#[cfg(test)]
mod test_upper_bounds {
    use super::*;

    fn quick_ratio(a: &str, b: &str) -> f32 {
        let (ca, la) = line_counts(a);
        let (cb, lb) = line_counts(b);
        quick_ratio_bound(&ca, la, &cb, lb)
    }

    #[test]
    fn real_quick_ratio_is_one_for_equal_lengths_all_match() {
        // Identical inputs: bound is 1.0 and the true ratio is 1.0.
        assert_eq!(real_quick_ratio(3, 3), 1.0);
    }

    #[test]
    fn both_bounds_are_at_least_the_true_ratio() {
        let pairs = [
            ("a\nb\nc\n", "a\nb\nc\n"),
            ("a\nb\nc\n", "a\nx\nc\n"),
            ("a\nb\nc\nd\n", "d\nc\nb\na\n"),
            ("a\nb\n", "a\nb\nc\nd\ne\n"),
            ("totally\ndifferent\n", "nothing\nmatches\nhere\n"),
        ];
        for (a, b) in pairs {
            let (_, la) = line_counts(a);
            let (_, lb) = line_counts(b);
            let truth = get_similarity_ratio(a, b);
            let rq = real_quick_ratio(la, lb);
            let q = quick_ratio(a, b);
            assert!(
                rq >= truth - 1e-6,
                "real_quick {rq} < truth {truth} for {a:?},{b:?}"
            );
            assert!(
                q >= truth - 1e-6,
                "quick {q} < truth {truth} for {a:?},{b:?}"
            );
            assert!(q <= rq + 1e-6, "quick {q} should be <= real_quick {rq}");
        }
    }

    #[test]
    fn empty_inputs_bound_is_one() {
        assert_eq!(real_quick_ratio(0, 0), 1.0);
        assert_eq!(quick_ratio("", ""), 1.0);
    }
}

#[cfg(test)]
mod test_topn {
    use super::*;

    fn fc(path: &str, ratio: f32) -> FileComparison {
        FileComparison {
            path: PathBuf::from(path),
            similarity_ratio: ratio,
            content: String::new(),
        }
    }

    #[test]
    fn keeps_highest_ratios_sorted_descending() {
        let mut top = TopN::new(2);
        top.push(0, fc("a", 0.1));
        top.push(1, fc("b", 0.9));
        top.push(2, fc("c", 0.5));
        let out = top.into_sorted_vec();
        assert_eq!(
            out.iter()
                .map(|c| c.path.to_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["b", "c"]
        );
    }

    #[test]
    fn ties_break_by_walk_index_ascending() {
        let mut top = TopN::new(2);
        // Equal ratios; lower walk index must win and sort first.
        top.push(5, fc("late", 0.5));
        top.push(1, fc("early", 0.5));
        top.push(9, fc("latest", 0.5));
        let out = top.into_sorted_vec();
        assert_eq!(
            out.iter()
                .map(|c| c.path.to_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["early", "late"]
        );
    }

    #[test]
    fn should_compute_is_false_only_when_full_and_below_threshold() {
        let mut top = TopN::new(2);
        assert!(top.should_compute(0.0), "not full: always compute");
        top.push(0, fc("a", 0.4));
        top.push(1, fc("b", 0.6));
        // Full; threshold is the lowest kept ratio, 0.4.
        assert!(!top.should_compute(0.3), "below threshold: prune");
        assert!(
            top.should_compute(0.4),
            "equal to threshold: compute (tie safety)"
        );
        assert!(top.should_compute(0.5), "above threshold: compute");
    }

    #[test]
    fn capacity_zero_never_computes_and_is_empty() {
        let mut top = TopN::new(0);
        assert!(!top.should_compute(1.0));
        top.push(0, fc("a", 1.0));
        assert!(top.into_sorted_vec().is_empty());
    }

    #[test]
    fn merge_keeps_global_top_n() {
        let mut a = TopN::new(2);
        a.push(0, fc("a", 0.9));
        a.push(1, fc("b", 0.2));
        let mut b = TopN::new(2);
        b.push(2, fc("c", 0.7));
        b.push(3, fc("d", 0.1));
        let out = a.merge(b).into_sorted_vec();
        assert_eq!(
            out.iter()
                .map(|c| c.path.to_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["a", "c"]
        );
    }
}
