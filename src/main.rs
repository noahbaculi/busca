use busca::{FileMatch, FileMatches};
use clap::Parser;
use console::{style, Style};
use indicatif::ProgressBar;
use indicatif::ProgressState;
use indicatif::ProgressStyle;
use inquire::Select;
use similar::{ChangeTag, TextDiff};
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process;
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
}

#[derive(Debug)]
struct Args {
    ref_file_path: PathBuf,
    search_path: PathBuf,
    extensions: Option<Vec<String>>,
    max_lines: u32,
    count: u8,
}

fn validate_args(input_args: InputArgs) -> Args {
    if !input_args.ref_file_path.is_file() {
        panic!(
            "The reference file path '{}' could not be found",
            input_args.ref_file_path.display()
        );
    }

    // Assign to CWD if the arg is not given
    let search_path = input_args
        .search_path
        .clone()
        .unwrap_or(env::current_dir().unwrap());

    if !search_path.is_file() & !search_path.is_dir() {
        panic!(
            "The search path '{}' could not be found",
            search_path.display()
        );
    }

    let mut count = input_args.count;
    if (count == 0) | (count > 200) {
        count = 10
    }

    Args {
        ref_file_path: input_args.ref_file_path,
        search_path,
        extensions: input_args.ext,
        max_lines: input_args.max_lines,
        count,
    }
}

pub fn run_search(
    ref_file_path: &PathBuf,
    search_path: &PathBuf,
    extensions: &Option<Vec<String>>,
    max_lines: &u32,
) -> Result<FileMatches, Box<dyn Error>> {
    let mut path_to_perc_shared = FileMatches(Vec::new());

    let ref_lines = fs::read_to_string(ref_file_path).unwrap();

    let search_root = search_path.clone().into_os_string().into_string().unwrap();

    let num_files = WalkDir::new(&search_root).into_iter().count();

    // Create progress bar
    let progress_bar = ProgressBar::new(num_files.try_into().unwrap());
    progress_bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos} / {human_len} files ({percent}%)",
        )
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
        })
        .progress_chars("#>-"),
    );

    // Walk through search path
    for dir_entry_result in WalkDir::new(&search_root) {
        progress_bar.inc(1);
        if dir_entry_result.is_err() {
            continue;
        }

        let path_in_dir = dir_entry_result.unwrap().into_path();

        // Skip paths that are not files
        if !path_in_dir.is_file() {
            continue;
        }

        let extension = path_in_dir
            .extension()
            .unwrap_or(OsStr::new(""))
            .to_os_string()
            .into_string()
            .unwrap_or("".to_string());

        if (extensions.is_some()) && !(extensions.clone().unwrap().contains(&extension)) {
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

        let perc_shared = busca::get_perc_shared_lines(&ref_lines, &comp_lines);
        path_to_perc_shared.push(FileMatch {
            path: path_in_dir.clone(),
            perc_shared,
        });
    }
    progress_bar.finish();

    Ok(path_to_perc_shared)
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

fn main() {
    let input_args = InputArgs::parse();

    let args = validate_args(input_args);

    let mut search_results = run_search(
        &args.ref_file_path,
        &args.search_path,
        &args.extensions,
        &args.max_lines,
    )
    .unwrap();

    search_results.sort_by(|a, b| b.perc_shared.partial_cmp(&a.perc_shared).unwrap());

    search_results.truncate(args.count.into());

    let file_matches = &search_results.to_string();
    let mut grid_options: Vec<_> = file_matches.split('\n').collect();

    // Remove the last new line
    grid_options.remove(grid_options.len() - 1);
    // dbg!(&grid_options);

    if grid_options.len() == 0 {
        println!("No files found.");
        process::exit(0);
    }

    let ans = Select::new("Select a file to compare:", grid_options)
        .raw_prompt()
        .expect("Prompt response should be valid");

    let selected_search = &search_results[*&ans.index];
    let selected_search_path = &selected_search.path;

    let ref_lines = fs::read_to_string(&args.ref_file_path).unwrap();
    let comp_lines = fs::read_to_string(&selected_search_path).unwrap();

    let diff = TextDiff::from_lines(&ref_lines, &comp_lines);

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
