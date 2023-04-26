use busca::{FileMatch, FileMatches};
// use clap::Arg;
use clap::Parser;
use console::{style, Style};
use indicatif::ProgressIterator;
// use indicatif::ProgressState;
use indicatif::ProgressStyle;
use inquire::InquireError;
use inquire::Select;
// use pariter::IteratorExt::*;
// use rayon::iter::ParallelBridge;
// use rayon::prelude::*;
use similar::{ChangeTag, TextDiff};
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
// use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process;
use std::process::exit;
// use std::sync::mpsc::channel;
use walkdir::WalkDir;

/// Simple utility to find the closest matches to a reference file in a
/// directory based on the number of lines in the reference file that exist in
/// each compared file.
#[derive(Parser, Debug)]
#[command(author="Noah Baculi", version, about, long_about = None)]

struct InputArgs {
    /// Local or absolute path to the reference comparison file
    ref_file_path: PathBuf,

    /// Directory or file in which to search. Defaults to CWD
    #[arg(short, long)]
    search_path: Option<PathBuf>,

    /// File extensions to include in the search. ex: `-e py -e json`. Defaults to all files with
    /// valid UTF-8 contents
    #[arg(short, long)]
    ext: Option<Vec<String>>,

    /// The number of lines to consider when comparing files. Files with more
    /// lines will be skipped.
    #[arg(short, long, default_value_t = 10_000)]
    max_lines: u32,

    /// Number of results to display
    #[arg(short, long, default_value_t = 10)]
    count: u8,

    /// Print all files being considered for comparison
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, PartialEq)]
struct Args {
    ref_file_path: PathBuf,
    search_path: PathBuf,
    extensions: Option<Vec<String>>,
    max_lines: u32,
    count: u8,
    verbose: bool,
}

/// Validates input args.
fn validate_args(input_args: InputArgs) -> Result<Args, String> {
    if !input_args.ref_file_path.is_file() {
        return Err(format!(
            "The reference file path '{}' could not be found.",
            input_args.ref_file_path.display()
        ));
    }

    // Assign to CWD if the arg is not given
    let search_path = input_args
        .search_path
        .clone()
        .unwrap_or(env::current_dir().unwrap());

    if !search_path.is_file() & !search_path.is_dir() {
        return Err(format!(
            "The search path '{}' could not be found.",
            search_path.display()
        ));
    }

    Ok(Args {
        ref_file_path: input_args.ref_file_path,
        search_path,
        extensions: input_args.ext,
        max_lines: input_args.max_lines,
        count: input_args.count,
        verbose: input_args.verbose,
    })
}
#[cfg(test)]
mod test_validate_args {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            ref_file_path: PathBuf::from(r"sample_dir_hello_world/file_2.py"),
            search_path: PathBuf::from(r"sample_dir_hello_world"),
            extensions: Some(vec!["py".to_string(), "json".to_string()]),
            max_lines: 5000,
            count: 8,
            verbose: true,
        }
    }

    #[test]
    fn valid_args() {
        let valid_args = get_valid_args();

        // No changes are made to parameters
        let input_args = InputArgs {
            ref_file_path: valid_args.ref_file_path.clone(),
            search_path: Some(valid_args.search_path.clone()),
            ext: valid_args.extensions.clone(),
            max_lines: valid_args.max_lines,
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            validate_args(input_args),
            Ok(Args {
                ref_file_path: valid_args.ref_file_path.clone(),
                search_path: valid_args.search_path.clone(),
                extensions: valid_args.extensions.clone(),
                max_lines: valid_args.max_lines,
                count: valid_args.count,
                verbose: valid_args.verbose,
            })
        );
    }

    #[test]
    fn override_args() {
        let valid_args = get_valid_args();
        let input_args = InputArgs {
            ref_file_path: valid_args.ref_file_path.clone(),
            search_path: None,
            ext: valid_args.extensions.clone(),
            max_lines: valid_args.max_lines,
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            validate_args(input_args),
            Ok(Args {
                ref_file_path: valid_args.ref_file_path.clone(),
                search_path: env::current_dir().unwrap(),
                extensions: valid_args.extensions.clone(),
                max_lines: valid_args.max_lines,
                count: valid_args.count,
                verbose: valid_args.verbose,
            })
        );
    }

    #[test]
    fn nonexistent_reference_path() {
        let valid_args = get_valid_args();
        let input_args_wrong_ref_file = InputArgs {
            ref_file_path: PathBuf::from(r"nonexistent_path"),
            search_path: Some(valid_args.search_path.clone()),
            ext: valid_args.extensions.clone(),
            max_lines: valid_args.max_lines,
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            validate_args(input_args_wrong_ref_file),
            Err("The reference file path 'nonexistent_path' could not be found.".to_string())
        );
    }

    #[test]
    fn nonexistent_search_path() {
        let valid_args = get_valid_args();
        let input_args_wrong_ref_file = InputArgs {
            ref_file_path: valid_args.ref_file_path.clone(),
            search_path: Some(PathBuf::from(r"nonexistent_path")),
            ext: valid_args.extensions.clone(),
            max_lines: valid_args.max_lines,
            count: valid_args.count,
            verbose: valid_args.verbose,
        };
        assert_eq!(
            validate_args(input_args_wrong_ref_file),
            Err("The search path 'nonexistent_path' could not be found.".to_string())
        );
    }
}

fn compare_file(
    dir_entry_result: Result<walkdir::DirEntry, walkdir::Error>,
    args: &Args,
    ref_lines: &str,
) -> Option<FileMatch> {

    if dir_entry_result.is_err() {
        if args.verbose {
            println!(
                "{} | skipped since file cannot be read.",
                dir_entry_result.unwrap().into_path().display()
            );
        }
        return None;
    }

    let path_in_dir = dir_entry_result.unwrap().into_path();
    if args.verbose {
        print!("{}", &path_in_dir.display());
    }

    // Skip paths that are not files
    if !path_in_dir.is_file() {
        if args.verbose {
            println!(" | skipped since it is not a file.");
        }
        return None;
    }

    // Skip paths that do not match the extensions
    let extension = path_in_dir
        .extension()
        .unwrap_or(OsStr::new(""))
        .to_os_string()
        .into_string()
        .unwrap_or("".to_string());

    if (args.extensions.is_some()) && !(args.extensions.clone().unwrap().contains(&extension)) {
        if args.verbose {
            println!(" | skipped since it does not match the extension filter.");
        }
        return None;
    }

    let comp_reader = fs::read_to_string(&path_in_dir);
    let comp_lines = match comp_reader {
        Ok(lines) => lines,
        Err(error) => match error.kind() {
            ErrorKind::InvalidData => return None,
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
        path: path_in_dir,
        perc_shared,
    })
}
#[cfg(test)]
mod test_compare_file {
    use super::*;

    fn get_valid_args() -> Args {
        Args {
            ref_file_path: PathBuf::from(r"sample_dir_hello_world/file_2.py"),
            search_path: PathBuf::from(r"sample_dir_hello_world"),
            extensions: None,
            max_lines: 5000,
            count: 8,
            verbose: true,
        }
    }

    #[test]
    fn skip_directory() {
        let valid_args = get_valid_args();

        let ref_lines = fs::read_to_string(PathBuf::from(
            r"sample_dir_hello_world/nested_dir/sample_python_file_3.py",
        ))
        .unwrap();

        let dir_entry_result = WalkDir::new("sample_dir_hello_world")
            .into_iter()
            .next()
            .unwrap();

        let file_comparison =
            compare_file(dir_entry_result, &valid_args, &ref_lines);

        assert_eq!(file_comparison, None);
    }

    #[test]
    fn same_file_comparison() {
        let valid_args = get_valid_args();

        let file_path_str = "sample_dir_hello_world/nested_dir/sample_python_file_3.py";

        let ref_lines = fs::read_to_string(PathBuf::from(file_path_str)).unwrap();

        let dir_entry_result = WalkDir::new(file_path_str).into_iter().next().unwrap();

        let file_comparison =
            compare_file(dir_entry_result, &valid_args, &ref_lines);

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
            r"sample_dir_hello_world/nested_dir/sample_python_file_3.py",
        ))
        .unwrap();

        let comp_path_str = "sample_dir_hello_world/file_1.py";

        let dir_entry_result = WalkDir::new(comp_path_str).into_iter().next().unwrap();

        let file_comparison =
            compare_file(dir_entry_result, &valid_args, &ref_lines);

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(comp_path_str),
                perc_shared: 0.6
            })
        );
    }

    #[test]
    fn include_extensions() {
        // TODO
        let mut valid_args = get_valid_args();
        valid_args.extensions = Some(vec!["json".to_string()]);

        let comp_path_str = "sample_dir_hello_world/nested_dir/sample_json.json";

        let dir_entry_result = WalkDir::new(comp_path_str).into_iter().next().unwrap();

        let file_comparison =
            compare_file(dir_entry_result, &valid_args, "");

        assert_eq!(
            file_comparison,
            Some(FileMatch {
                path: PathBuf::from(comp_path_str),
                perc_shared: 0.0
            })
        );
    }
    #[test]
    fn exclude_extensions() {
        let mut valid_args = get_valid_args();
        valid_args.extensions = Some(vec!["py".to_string()]);

        let comp_path_str = "sample_dir_hello_world/nested_dir/sample_json.json";

        let dir_entry_result = WalkDir::new(comp_path_str).into_iter().next().unwrap();

        let file_comparison =
            compare_file(dir_entry_result, &valid_args, "");

        assert_eq!(
            file_comparison,
            None
        );
    }
}

fn run_search(args: &Args) -> Result<FileMatches, Box<dyn Error>> {
    let ref_lines = fs::read_to_string(&args.ref_file_path).unwrap();

    let search_root = args
        .search_path
        .clone()
        .into_os_string()
        .into_string()
        .unwrap();

    // Create progress bar style
    let progress_bar_style = 
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos} / {human_len} files ({percent}%)",
        )
        .unwrap()
        .progress_chars("#>-");


    // Walk through search path
    let mut file_match_vec: Vec<FileMatch> = WalkDir::new(search_root)
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .progress_with_style(progress_bar_style)
        .filter_map(|dir_entry_result| {
            compare_file(dir_entry_result, args, &ref_lines)
        })
        .collect();

    // Sort by percent match
    file_match_vec.sort_by(|a, b| b.perc_shared.partial_cmp(&a.perc_shared).unwrap());

    // Keep the top matches
    file_match_vec.truncate(args.count.into());

    Ok(busca::FileMatches(file_match_vec))
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

fn graceful_panic(error_str: String) -> ! {
    eprintln!("{}", error_str);
    exit(1);
}

fn main() {
    let input_args = InputArgs::parse();

    let args = match validate_args(input_args) {
        Ok(args) => args,
        Err(err) => graceful_panic(err),
    };

    let search_results = run_search(&args).unwrap();

    let file_matches = &search_results.to_string();
    let mut grid_options: Vec<_> = file_matches.split('\n').collect();

    // Remove the last new line
    grid_options.remove(grid_options.len() - 1);

    if grid_options.is_empty() {
        println!("No files found that match the criteria.");
        process::exit(0);
    }

    let ans = match Select::new("Select a file to compare:", grid_options).raw_prompt() {
        Ok(answer) => answer,
        Err(InquireError::OperationCanceled) => exit(0),
        Err(err) => graceful_panic(err.to_string()),
    };

    let selected_search = &search_results[ans.index];
    let selected_search_path = &selected_search.path;
    let ref_lines = fs::read_to_string(&args.ref_file_path).unwrap();
    let comp_lines = fs::read_to_string(selected_search_path).unwrap();
    output_detailed_diff(&ref_lines, &comp_lines);
}

fn output_detailed_diff(ref_lines: &str, comp_lines: &str) {
    let diff = TextDiff::from_lines(ref_lines, comp_lines);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
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
