use similar::{ChangeTag, TextDiff};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use term_grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};
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
#[derive(Debug, Clone)]
pub struct FileMatch {
    pub path: PathBuf,
    pub perc_shared: f32,
}
#[derive(Debug, Clone)]
pub struct FileMatches(Vec<FileMatch>);

// impl FileMatches {}

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
    pub fn max_path_width(&self) -> Option<usize> {
        self.iter()
            .map(|x| x.path.display().to_string().chars().count())
            .max()
    }
}

impl fmt::Display for FileMatches {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut grid = Grid::new(GridOptions {
            filling: Filling::Spaces(2),
            direction: Direction::LeftToRight,
        });

        for path_and_perc in self.iter() {
            grid.add(Cell::from(path_and_perc.path.display().to_string()));

            let visual_indicator = "+".repeat((path_and_perc.perc_shared * 10.0).round() as usize);
            let vis_cell = Cell::from(visual_indicator);
            grid.add(vis_cell);

            let perc_str = format!("{:.1}%", (path_and_perc.perc_shared * 100.0));
            let mut perc_cell = Cell::from(perc_str);
            perc_cell.alignment = Alignment::Right;
            grid.add(perc_cell);
        }

        let disp = grid.fit_into_columns(3);

        let grid_str = disp.to_string();
        write!(f, "{}", grid_str)
    }
}

pub fn run_search(
    ref_file_path: &PathBuf,
    search_path: &PathBuf,
    extensions: &Vec<String>,
    max_lines: &u32,
) -> Result<FileMatches, Box<dyn Error>> {
    let mut path_to_perc_shared = FileMatches(Vec::new());

    let ref_lines = fs::read_to_string(ref_file_path).unwrap();

    let search_root = search_path.clone().into_os_string().into_string().unwrap();

    let test = WalkDir::new(&search_root).into_iter().count();

    dbg!(test);

    // Walk through search path
    let walkdir = WalkDir::new(&search_root)
        .into_iter()
        .filter_map(|e| e.ok());

    for dir_entry_result in walkdir {
        let path_in_dir = dir_entry_result.into_path();

        // Skip paths that are not files
        if !path_in_dir.is_file() {
            continue;
        }

        let extension = path_in_dir
            .extension()
            .unwrap_or(OsStr::new(""))
            .to_os_string();

        if !(extensions.contains(&extension.into_string().unwrap())) {
            continue;
        }

        let comp_reader = fs::read_to_string(&path_in_dir);
        let comp_lines = match comp_reader {
            Ok(lines) => lines,
            Err(error) => match error.kind() {
                ErrorKind::InvalidData => continue,
                other_error => panic!("{:?}", other_error),
            },
        };

        let num_comp_lines = comp_lines.clone().lines().count();

        if (num_comp_lines > *max_lines as usize) | (num_comp_lines == 0) {
            continue;
        }

        dbg!(&path_in_dir);

        let perc_shared = get_perc_shared_lines(&ref_lines, &comp_lines);
        path_to_perc_shared.push(FileMatch {
            path: path_in_dir.clone(),
            perc_shared,
        });
    }

    Ok(path_to_perc_shared)
}
