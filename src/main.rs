use busca::format_file_comparisons;
use busca::{run_search_with_progress, Args, FileComparison};
use clap::Parser;
use console::{style, Style};
use indicatif::ProgressStyle;
use inquire::{InquireError, Select};
use similar::{ChangeTag, TextDiff};
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

/// Output error to the std err and exit with status code 1.
fn graceful_panic(error_str: &str) -> ! {
    eprintln!("{}", error_str);
    std::process::exit(1);
}

fn parse_similarity_ratio(s: &str) -> Result<f32, String> {
    let v: f32 = s
        .parse()
        .map_err(|e: std::num::ParseFloatError| e.to_string())?;
    if v.is_nan() {
        return Err("must be a number, got NaN".to_owned());
    }
    if !(0.0..=1.0).contains(&v) {
        return Err(format!("must be in [0.0, 1.0], got {v}"));
    }
    Ok(v)
}

fn main() {
    let input_args = InputArgs::parse();
    let min_similarity_ratio = input_args.min_similarity_ratio;
    let user_count = input_args.count;

    let args = match input_args.into_args() {
        Ok(args) => args,
        Err(err_str) => graceful_panic(&err_str),
    };

    let raw_comparisons = match cli_run_search(&args) {
        Ok(search_results) => search_results,
        Err(err_str) => graceful_panic(&err_str),
    };

    let mut file_comparisons = apply_min_similarity_ratio(raw_comparisons, min_similarity_ratio);
    file_comparisons.truncate(user_count);

    if file_comparisons.is_empty() {
        println!("No files found that match the criteria.");
        std::process::exit(0);
    }

    let file_comparisons_output = format_file_comparisons(&file_comparisons);
    let grid_options: Vec<&str> = file_comparisons_output.split('\n').collect();

    if !interactive_input_mode() {
        println!("{}", file_comparisons_output);
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

    let selected_file_comparison = &file_comparisons[ans.index];
    let selected_file_comparison_path = &selected_file_comparison.path;
    let candidate_content = match fs::read_to_string(selected_file_comparison_path) {
        Ok(candidate_content) => candidate_content,
        Err(err) => graceful_panic(&err.to_string()),
    };
    output_detailed_diff(&args.reference_string, &candidate_content);
}

/// Simple utility to search for files with content that most closely match the lines of a reference string.
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

    /// The maximum number of lines a candidate file may have. Candidates with
    /// more lines (or zero lines) are skipped entirely.
    #[arg(short, long, default_value_t = 10_000)]
    max_file_lines: usize,

    /// Globs that qualify a file for comparison
    #[arg(short, long)]
    include_glob: Option<Vec<String>>,

    /// Globs that disqualify a file from comparison
    #[arg(short = 'x', long)]
    exclude_glob: Option<Vec<String>>,

    /// Number of results to display
    #[arg(short, long, default_value_t = 10)]
    count: usize,

    /// Drop comparisons whose similarity ratio is below this value (in [0.0, 1.0]).
    /// Applied after sorting and before --count truncation.
    #[arg(long, value_parser = parse_similarity_ratio)]
    min_similarity_ratio: Option<f32>,
}

impl InputArgs {
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
                    Err(e) => return Err(e.to_string()),
                    Ok(s) => s,
                },
            },
            None => get_piped_input()?,
        };

        let search_path = match self.search_path {
            Some(p) => p,
            None => match env::current_dir() {
                Ok(p) => p,
                Err(e) => return Err(e.to_string()),
            },
        };

        Args::new(
            reference_string,
            search_path,
            Some(self.max_file_lines),
            None, // count applied in main after filtering
            self.include_glob.unwrap_or_default(),
            self.exclude_glob.unwrap_or_default(),
        )
        .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod test_input_args_validation {
    use super::*;

    fn get_valid_args() -> Args {
        Args::new(
            fs::read_to_string("sample_dir_hello_world/file_3.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            Some(8),
            vec!["*.py".into()],
            vec!["*.yml".into()],
        )
        .unwrap()
    }

    #[test]
    fn valid_args() {
        let valid_args = get_valid_args();

        // No changes are made to parameters
        let input_args = InputArgs {
            ref_file_path: Some(PathBuf::from("sample_dir_hello_world/file_3.py")),
            search_path: Some(valid_args.search_path.clone()),
            max_file_lines: valid_args.max_file_lines.unwrap(),
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count.unwrap(),
            min_similarity_ratio: None,
        };
        assert_eq!(
            input_args.into_args(),
            Ok(Args::new(
                valid_args.reference_string.clone(),
                valid_args.search_path.clone(),
                valid_args.max_file_lines,
                None,
                vec!["*.py".into()],
                vec!["*.yml".into()],
            )
            .unwrap())
        );
    }

    #[test]
    fn missing_optional_args() {
        let valid_args = get_valid_args();
        let input_args = InputArgs {
            ref_file_path: Some(PathBuf::from("sample_dir_hello_world/file_3.py")),
            search_path: None,
            max_file_lines: valid_args.max_file_lines.unwrap(),
            include_glob: None,
            exclude_glob: None,
            count: valid_args.count.unwrap(),
            min_similarity_ratio: None,
        };
        assert_eq!(
            input_args.into_args(),
            Ok(Args::new(
                valid_args.reference_string.clone(),
                env::current_dir().unwrap(),
                valid_args.max_file_lines,
                None,
                vec![],
                vec![],
            )
            .unwrap())
        );
    }

    #[test]
    fn nonexistent_reference_path() {
        let valid_args = get_valid_args();
        let input_args_wrong_ref_file = InputArgs {
            ref_file_path: Some(PathBuf::from("nonexistent_path")),
            search_path: Some(valid_args.search_path.clone()),
            max_file_lines: valid_args.max_file_lines.unwrap(),
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count.unwrap(),
            min_similarity_ratio: None,
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
            max_file_lines: valid_args.max_file_lines.unwrap(),
            include_glob: Some(vec!["*.py".to_owned()]),
            exclude_glob: Some(vec!["*.yml".to_owned()]),
            count: valid_args.count.unwrap(),
            min_similarity_ratio: None,
        };
        assert_eq!(
            input_args_wrong_ref_file.into_args(),
            Err("search path not found: nonexistent_path".to_owned())
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
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

fn cli_run_search(args: &Args) -> Result<Vec<FileComparison>, String> {
    let style_result = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos} / {human_len} files ({percent}%)",
    );

    let bar = indicatif::ProgressBar::new(0);
    if let Ok(style) = style_result {
        bar.set_style(style.progress_chars("#>-"));
    }

    let result = run_search_with_progress(args, |done, total| {
        if bar.length() != Some(total) {
            bar.set_length(total);
        }
        bar.set_position(done);
    });
    bar.finish_and_clear();
    result.map_err(|e| e.to_string())
}

fn output_detailed_diff(reference_string: &str, candidate_content: &str) {
    let diff = TextDiff::from_lines(reference_string, candidate_content);

    let grouped_operations = diff.grouped_ops(3);

    if grouped_operations.is_empty() {
        println!("The sequences are identical.");
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

fn apply_min_similarity_ratio(
    mut comparisons: Vec<FileComparison>,
    min: Option<f32>,
) -> Vec<FileComparison> {
    if let Some(min) = min {
        comparisons.retain(|c| c.similarity_ratio >= min);
    }
    comparisons
}

#[cfg(test)]
mod test_cli_run_search {
    use super::*;

    fn get_valid_args() -> Args {
        Args::new(
            fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            Some(2),
            vec!["*.py".into()],
            vec!["*.yml".into()],
        )
        .unwrap()
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
        assert_eq!(cli_run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn include_glob() {
        let valid_args = Args::new(
            fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            Some(2),
            vec!["*.json".into()],
            vec!["*.yml".into()],
        )
        .unwrap();

        let expected = vec![FileComparison {
            path: PathBuf::from("sample_dir_hello_world/nested_dir/sample_json.json"),
            similarity_ratio: 0.0,
            content: fs::read_to_string("sample_dir_hello_world/nested_dir/sample_json.json")
                .unwrap(),
        }];
        assert_eq!(cli_run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn exclude_glob() {
        let valid_args = Args::new(
            fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            Some(2),
            vec!["*.py".into()],
            vec!["*.json".into()],
        )
        .unwrap();

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
        assert_eq!(cli_run_search(&valid_args).unwrap(), expected);
    }

    #[test]
    fn min_similarity_ratio_filters_below_threshold() {
        let input = vec![
            FileComparison {
                path: PathBuf::from("a"),
                similarity_ratio: 0.9,
                content: String::new(),
            },
            FileComparison {
                path: PathBuf::from("b"),
                similarity_ratio: 0.2,
                content: String::new(),
            },
            FileComparison {
                path: PathBuf::from("c"),
                similarity_ratio: 0.0,
                content: String::new(),
            },
        ];
        let filtered = apply_min_similarity_ratio(input.clone(), Some(0.25));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, PathBuf::from("a"));

        let unfiltered = apply_min_similarity_ratio(input, None);
        assert_eq!(unfiltered.len(), 3);
    }

    #[test]
    fn sort_then_filter_then_truncate_pipeline() {
        let args = Args::new(
            fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            None,
            vec!["*.py".into()],
            vec![],
        )
        .unwrap();

        let raw = cli_run_search(&args).unwrap();
        for pair in raw.windows(2) {
            assert!(
                pair[0].similarity_ratio >= pair[1].similarity_ratio,
                "cli_run_search must return descending ratios, got {:?} then {:?}",
                pair[0].similarity_ratio,
                pair[1].similarity_ratio,
            );
        }

        let threshold = 0.25_f32;
        let user_count = 1_usize;
        let mut filtered = apply_min_similarity_ratio(raw.clone(), Some(threshold));
        filtered.truncate(user_count);

        assert!(filtered.len() <= user_count);
        for c in &filtered {
            assert!(
                c.similarity_ratio >= threshold,
                "{} = {} < {}",
                c.path.display(),
                c.similarity_ratio,
                threshold,
            );
        }
        let expected_full: Vec<_> = raw
            .iter()
            .filter(|c| c.similarity_ratio >= threshold)
            .take(user_count)
            .cloned()
            .collect();
        assert_eq!(filtered, expected_full);

        let none_left = apply_min_similarity_ratio(raw.clone(), Some(1.0001));
        assert!(none_left.is_empty(), "threshold >1 must filter everything");
    }
}

#[cfg(test)]
mod test_parse_similarity_ratio {
    use super::parse_similarity_ratio;

    #[test]
    fn accepts_in_range() {
        assert_eq!(parse_similarity_ratio("0.0"), Ok(0.0));
        assert_eq!(parse_similarity_ratio("1.0"), Ok(1.0));
        assert_eq!(parse_similarity_ratio("0.5"), Ok(0.5));
    }

    #[test]
    fn rejects_above_one() {
        assert!(parse_similarity_ratio("1.0001").is_err());
        assert!(parse_similarity_ratio("2.0").is_err());
    }

    #[test]
    fn rejects_negative() {
        assert!(parse_similarity_ratio("-0.1").is_err());
    }

    #[test]
    fn rejects_nan() {
        assert!(parse_similarity_ratio("NaN").is_err());
        assert!(parse_similarity_ratio("nan").is_err());
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(parse_similarity_ratio("abc").is_err());
        assert!(parse_similarity_ratio("").is_err());
    }
}
