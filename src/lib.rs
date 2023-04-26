use similar::{ChangeTag, TextDiff};
use std::fmt;
use std::path::PathBuf;
use term_grid::{Alignment, Cell, Direction, Filling, Grid, GridOptions};

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

// #[cfg(test)]
// mod tests {
//     use std::{path::PathBuf, str::FromStr};

//     #[test]
//     fn test_drafter() {
//         // Space to draft test with syntax highlighting and rust-analyzer
//     }
// }
