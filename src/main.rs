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

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

/// Print an error message to stderr and exit with status code 2.
fn graceful_panic(error_str: &str) -> ! {
    eprintln!("{}", error_str);
    std::process::exit(2);
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

fn parse_count(s: &str) -> Result<usize, String> {
    let v: usize = s
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    if v == 0 {
        return Err("must be at least 1".to_owned());
    }
    Ok(v)
}

fn main() {
    let input_args = InputArgs::parse();

    let output_format = input_args.format;
    let with_content = input_args.with_content;
    let no_interactive = input_args.no_interactive;

    let args = match input_args.into_args() {
        Ok(args) => args,
        Err(err_str) => graceful_panic(&err_str),
    };

    let file_comparisons = match cli_run_search(&args) {
        Ok(search_results) => search_results,
        Err(err_str) => graceful_panic(&err_str),
    };

    if file_comparisons.is_empty() {
        eprintln!("No files found that match the criteria.");
        std::process::exit(1);
    }

    match output_format {
        OutputFormat::Json => {
            println!("{}", comparisons_to_json(&file_comparisons, with_content));
        }
        OutputFormat::Human => {
            let file_comparisons_output = format_file_comparisons(&file_comparisons);
            let is_tty = interactive_input_mode();
            let interactive = is_tty && !no_interactive;

            if !interactive {
                println!("{}", file_comparisons_output);
                // Explain the missing picker only on automatic fallback, and on
                // stderr so stdout stays clean for parsing.
                if !is_tty && !no_interactive {
                    eprintln!("Note: interactive prompt is not supported in this mode.");
                }
                return;
            }

            let grid_options: Vec<&str> = file_comparisons_output.split('\n').collect();
            let ans = match Select::new("Select a file to compare:", grid_options)
                .with_page_size(10)
                .raw_prompt()
            {
                Ok(answer) => answer,
                Err(InquireError::OperationCanceled) => std::process::exit(0),
                Err(err) => graceful_panic(&err.to_string()),
            };

            let selected_file_comparison = &file_comparisons[ans.index];
            // Reuse the content captured during the search rather than reading the
            // file again. That keeps the diff consistent with the ranked ratio and
            // avoids a second read that could fail if the file changed meanwhile.
            output_detailed_diff(&args.reference_string, &selected_file_comparison.content);
        }
    }
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
    #[arg(short, long, default_value_t = 10, value_parser = parse_count)]
    count: usize,

    /// Drop comparisons whose similarity ratio is below this value (in [0.0, 1.0]).
    /// Applied during the search, before the --count limit.
    #[arg(long, value_parser = parse_similarity_ratio)]
    min_similarity_ratio: Option<f32>,

    /// Output format for the ranked results
    #[arg(long, value_enum, default_value = "human")]
    format: OutputFormat,

    /// Include each file's content in JSON output. Ignored for the human format
    #[arg(long)]
    with_content: bool,

    /// Print the ranked list instead of launching the interactive picker
    #[arg(long)]
    no_interactive: bool,
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
            Some(self.count),
            self.min_similarity_ratio,
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
            None,
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
            format: OutputFormat::Human,
            with_content: false,
            no_interactive: false,
        };
        assert_eq!(
            input_args.into_args(),
            Ok(Args::new(
                valid_args.reference_string.clone(),
                valid_args.search_path.clone(),
                valid_args.max_file_lines,
                valid_args.count,
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
            format: OutputFormat::Human,
            with_content: false,
            no_interactive: false,
        };
        assert_eq!(
            input_args.into_args(),
            Ok(Args::new(
                valid_args.reference_string.clone(),
                env::current_dir().unwrap(),
                valid_args.max_file_lines,
                valid_args.count,
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
            format: OutputFormat::Human,
            with_content: false,
            no_interactive: false,
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
            format: OutputFormat::Human,
            with_content: false,
            no_interactive: false,
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

/// Returns `true` if stdin is a TTY (interactive).
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

/// One row of `--format json` output. Built in the CLI so the library and the
/// Python module never reference `serde`, which keeps it dead-code-eliminated
/// from the wheel (see ADR-0002).
#[derive(serde::Serialize)]
struct JsonComparison {
    path: String,
    similarity_ratio: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

/// Serialize the ranked comparisons as a pretty JSON array. `content` is
/// included only when `with_content` is set. Serialization of these plain
/// fields cannot fail.
fn comparisons_to_json(file_comparisons: &[FileComparison], with_content: bool) -> String {
    let rows: Vec<JsonComparison> = file_comparisons
        .iter()
        .map(|fc| JsonComparison {
            path: fc.path.display().to_string(),
            similarity_ratio: fc.similarity_ratio,
            content: with_content.then(|| fc.content.clone()),
        })
        .collect();
    serde_json::to_string_pretty(&rows).expect("JSON serialization of comparisons cannot fail")
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

#[cfg(test)]
mod test_cli_run_search {
    use super::*;

    fn get_valid_args() -> Args {
        Args::new(
            fs::read_to_string("sample_dir_hello_world/nested_dir/ref_B.py").unwrap(),
            PathBuf::from("sample_dir_hello_world"),
            Some(5000),
            Some(2),
            None,
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
            None,
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
            None,
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

#[cfg(test)]
mod test_parse_count {
    use super::parse_count;

    #[test]
    fn accepts_positive() {
        assert_eq!(parse_count("1"), Ok(1));
        assert_eq!(parse_count("10"), Ok(10));
    }

    #[test]
    fn rejects_zero() {
        assert!(parse_count("0").is_err());
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(parse_count("abc").is_err());
        assert!(parse_count("-1").is_err());
        assert!(parse_count("").is_err());
    }
}
