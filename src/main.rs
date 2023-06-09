use busca::{FileMatch, FileMatches};
use clap::Parser;
use console::{style, Style};
use glob::Pattern;
use indicatif::{ParallelProgressIterator, ProgressStyle};
use inquire::{InquireError, Select};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use similar::{ChangeTag, TextDiff};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Output error to the std err and exit with status code 1.
fn graceful_panic(error_str: &str) -> ! {
    eprintln!("{}", error_str);
    std::process::exit(1);
}
fn main() {
    let input_args = InputArgs::parse();

    let args = match input_args.into_args() {
        Ok(args) => args,
        Err(err_str) => graceful_panic(&err_str),
    };

    let search_results = match run_search(&args) {
        Ok(search_results) => search_results,
        Err(_) => todo!(),
    };

    let file_matches = &search_results.to_string();
    let grid_options: Vec<&str> = file_matches.split('\n').collect();

    if grid_options.is_empty() {
        println!("No files found that match the criteria.");
        std::process::exit(0);
    }

    if !interactive_input_mode() {
        println!("{}", file_matches);
        println!("\nNote: Interactive prompt is not supported in this mode.");
        return;
    }

    let ans = match Select::new("Select a file to compare:", grid_options)
        .with_page_size(10)
        .raw_prompt()
    {
        Ok(answer) => answer,
        Err(InquireError::OperationCanceled) => std::process::exit(0),
        Err(err) => graceful_panic(&err.to_string()),
    };

    let selected_search = &search_results[ans.index];
    let selected_search_path = &selected_search.path;
    let comp_lines = match fs::read_to_string(selected_search_path) {
        Ok(comp_lines) => comp_lines,
        Err(err) => graceful_panic(&err.to_string()),
    };
    output_detailed_diff(&args.reference_string, &comp_lines);
}

/// Simple utility to find the closest matches to a reference file or piped input
/// based on the number of lines in the reference that exist in each compared file.
#[derive(Parser, Debug)]
#[command(author="Noah Baculi", version, about, long_about = None, override_usage="\
    busca --ref-file-path <REF_FILE_PATH> [OPTIONS]\n       \
    <SomeCommand> | busca [OPTIONS]"
)]
struct InputArgs {
    /// Local or absolute path to the reference comparison file. Overrides any
    /// piped input
    #[arg(short, long)]
    ref_file_path: Option<PathBuf>,

    /// Directory or file in which to search. Defaults to CWD
    #[arg(short, long)]
    search_path: Option<PathBuf>,

    /// The number of lines to consider when comparing files. Files with more
    /// lines will be skipped.
    #[arg(short, long, default_value_t = 10_000)]
    max_lines: u32,

    /// Globs that qualify a file for comparison
    #[arg(short, long)]
    include_glob: Option<Vec<String>>,

    /// Globs that disqualify a file from comparison
    #[arg(short = 'x', long)]
    exclude_glob: Option<Vec<String>>,

    /// Number of results to display
    #[arg(short, long, default_value_t = 10)]
    count: u8,

    /// Print all files being considered for comparison
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, PartialEq)]
struct Args {
    reference_string: String,
    search_path: PathBuf,
    max_lines: u32,
    include_patterns: Option<Vec<Pattern>>,
    exclude_patterns: Option<Vec<Pattern>>,
    count: u8,
    verbose: bool,
}

impl InputArgs {
    // Consumes and validates InputArgs and returns Args
    pub fn into_args(self) -> Result<Args, String> {
        let reference_string = match self.ref_file_path {
            Some(ref_file_path) => match ref_file_path.is_file() {
                false => {
                    return Err(format!(
                        "The reference file path '{}' is not a file.",
                        ref_file_path.display()
                    ))
                }

                true => match fs::read_to_string(ref_file_path) {
                    Err(e) => return Err(format!("{:?}", e)),
                    Ok(ref_file_string) => ref_file_string,
                },
            },
            None => get_piped_input()?,
        };

        // Assign search_path to CWD if the arg is not given
        let search_path = match self.search_path {
            Some(input_search_path) => input_search_path,

            None => match env::current_dir() {
                Ok(cwd_path) => cwd_path,
                Err(e) => return Err(format!("{:?}", e)),
            },
        };

        if !search_path.is_file() & !search_path.is_dir() {
            return Err(format!(
                "The search path '{}' could not be found.",
                search_path.display()
            ));
        }

        fn parse_glob_pattern(pattern_string: &str) -> Pattern {
            match glob::Pattern::new(pattern_string) {
                Ok(pattern) => pattern,
                Err(e) => graceful_panic(&format!("{:?} for '{}'", e, pattern_string)),
            }
        }

        // Parse the include glob patterns from input args strings
        let include_patterns = self.include_glob.map(|include_substring_vec| {
            include_substring_vec
                .iter()
                .map(|include_substring| parse_glob_pattern(include_substring))
                .collect()
        });

        // Parse the exclude glob patterns from input args strings
        let exclude_patterns = self.exclude_glob.map(|exclude_substring_vec| {
            exclude_substring_vec
                .iter()
                .map(|exclude_substring| parse_glob_pattern(exclude_substring))
                .collect()
        });

        Ok(Args {
            reference_string,
            search_path,
            max_lines: self.max_lines,
            include_patterns,
            exclude_patterns,
            count: self.count,
            verbose: self.verbose,
        })
    }
}

#[cfg(test)]
mod test_input_args_validation {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            reference_string: fs::read_to_string("sample_dir_hello_world/file_3.py").unwrap(),
            search_path: PathBuf::from("sample_dir_hello_world"),
            max_lines: 5000,
            include_patterns: Some(vec![Pattern::new("*.py").unwrap()]),
            exclude_patterns: Some(vec![Pattern::new("*.yml").unwrap()]),
            count: 8,
            verbose: false,
        }
    }

    #[test]
    fn valid_args() {
        let valid_args = get_valid_args();

        // No changes are made to parameters
        let input_args = InputArgs {
            ref_file_path: Some(PathBuf::from("sample_dir_hello_world/file_3.py")),
            search_path: Some(valid_args.search_path.clone()),
            max_lines: valid_args.max_lines,
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            input_args.into_args(),
            Ok(Args {
                reference_string: valid_args.reference_string,
                search_path: valid_args.search_path.clone(),
                max_lines: valid_args.max_lines,
                include_patterns: valid_args.include_patterns.clone(),
                exclude_patterns: valid_args.exclude_patterns.clone(),
                count: valid_args.count,
                verbose: valid_args.verbose,
            })
        );
    }

    #[test]
    fn override_args() {
        let valid_args = get_valid_args();
        let input_args = InputArgs {
            ref_file_path: Some(PathBuf::from("sample_dir_hello_world/file_3.py")),
            search_path: None,
            max_lines: valid_args.max_lines,
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            input_args.into_args(),
            Ok(Args {
                reference_string: valid_args.reference_string,
                search_path: env::current_dir().unwrap(),
                max_lines: valid_args.max_lines,
                include_patterns: valid_args.include_patterns.clone(),
                exclude_patterns: valid_args.exclude_patterns.clone(),
                count: valid_args.count,
                verbose: valid_args.verbose,
            })
        );
    }

    #[test]
    fn nonexistent_reference_path() {
        let valid_args = get_valid_args();
        let input_args_wrong_ref_file = InputArgs {
            ref_file_path: Some(PathBuf::from("nonexistent_path")),
            search_path: Some(valid_args.search_path.clone()),
            max_lines: valid_args.max_lines,
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            input_args_wrong_ref_file.into_args(),
            Err("The reference file path 'nonexistent_path' is not a file.".to_owned())
        );
    }

    #[test]
    fn nonexistent_search_path() {
        let valid_args = get_valid_args();
        let input_args_wrong_ref_file = InputArgs {
            ref_file_path: Some(PathBuf::from("sample_dir_hello_world/file_3.py")),
            search_path: Some(PathBuf::from("nonexistent_path")),
            max_lines: valid_args.max_lines,
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            input_args_wrong_ref_file.into_args(),
            Err("The search path 'nonexistent_path' could not be found.".to_owned())
        );
    }
}

fn get_piped_input() -> Result<String, String> {
    use std::io::{self, BufRead};

    if interactive_input_mode() {
        return Err("No piped input was received. For more information, try '--help'.".to_owned());
    }

    let piped_input: String = io::stdin()
        .lock()
        .lines()
        .map(|l| l.unwrap_or("".to_owned()))
        .collect::<Vec<String>>()
        .join("\n");

    if piped_input.is_empty() {
        return Err("No piped input was received. For more information, try '--help'.".to_owned());
    }

    Ok(piped_input)
}

/// If the current stdin is a TTY (interactive)
fn interactive_input_mode() -> bool {
    atty::is(atty::Stream::Stdin)
}

fn run_search(args: &Args) -> Result<FileMatches, String> {
    // Create progress bar style
    let progress_bar_style_result = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos} / {human_len} files ({percent}%)",
        );

    let walkdir_vec = WalkDir::new(&args.search_path)
        .into_iter()
        .collect::<Vec<_>>();

    let mut file_match_vec: Vec<FileMatch> = match progress_bar_style_result {
        Ok(progress_bar_style) => walkdir_vec
            .par_iter()
            .progress_with_style(progress_bar_style.progress_chars("#>-"))
            .filter_map(|dir_entry_result| match dir_entry_result {
                Ok(dir_entry) => compare_file(dir_entry.path(), args, &args.reference_string),
                Err(_) => None,
            })
            .collect(),

        Err(_) => {
            println!(
                "The progress bar could not be configured. Launching search without feedback. Comparing {} files...", walkdir_vec.len()
            );
            walkdir_vec
                .par_iter()
                .filter_map(|dir_entry_result| match dir_entry_result {
                    Ok(dir_entry) => compare_file(dir_entry.path(), args, &args.reference_string),
                    Err(_) => None,
                })
                .collect()
        }
    };

    // Sort by percent match
    file_match_vec.sort_by(|a, b| {
        b.perc_shared
            .partial_cmp(&a.perc_shared)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Keep the top matches
    file_match_vec.truncate(args.count.into());

    Ok(busca::FileMatches(file_match_vec))
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
            verbose: false,
        }
    }

    #[test]
    fn normal_search() {
        let valid_args = get_valid_args();

        let expected = busca::FileMatches(vec![
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

        let expected = busca::FileMatches(vec![FileMatch {
            path: PathBuf::from("sample_dir_hello_world/nested_dir/sample_json.json"),
            perc_shared: 0.0,
        }]);
        assert_eq!(run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn exclude_glob() {
        let mut valid_args = get_valid_args();
        valid_args.exclude_patterns = Some(vec![Pattern::new("*.json").unwrap()]);

        let expected = busca::FileMatches(vec![
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

fn compare_file(comp_path: &Path, args: &Args, ref_lines: &str) -> Option<FileMatch> {
    if args.verbose {
        print!("{}", &comp_path.display());
    }

    // Skip paths that are not files
    if !comp_path.is_file() {
        if args.verbose {
            println!(" | skipped since it is not a file.");
        }
        return None;
    }

    // Skip paths that do not contain the include glob pattern
    match &args.include_patterns {
        Some(include_pattern_vec) => {
            let contains_glob_pattern = include_pattern_vec
                .iter()
                .any(|include_pattern| include_pattern.matches_path(comp_path));

            if !contains_glob_pattern {
                if args.verbose {
                    println!(" | skipped since it does not contain the include glob pattern.");
                }
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
                if args.verbose {
                    println!(" | skipped since it contains the exclude glob pattern.");
                }
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
        if args.verbose {
            println!(" | skipped since it exceeds the maximum line limit.");
        }
        return None;
    }

    let perc_shared = busca::get_perc_shared_lines(ref_lines, &comp_lines);

    // Print new line after the file path print if file was compared.
    if args.verbose {
        println!();
    }

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
            verbose: false,
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

fn output_detailed_diff(ref_lines: &str, comp_lines: &str) {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    let grouped_operations = diff.grouped_ops(3);

    if grouped_operations.is_empty() {
        println!("The comparables are identical.");
        return;
    }

    for (idx, group) in grouped_operations.iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                print!(
                    "{} {} {} |",
                    style(Line(change.old_index())).dim(),
                    style(Line(change.new_index())).dim(),
                    s.apply_to(sign).bold(),
                );
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        print!("{}", s.apply_to(value).underlined().on_black());
                    } else {
                        print!("{}", s.apply_to(value));
                    }
                }
                if change.missing_newline() {
                    println!();
                }
            }
        }
    }
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}
