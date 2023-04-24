use clap::Parser;
use console::{style, Style};
use inquire::Select;
use similar::{ChangeTag, TextDiff};
use std::fmt;
use std::{env, fs, path::PathBuf};

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

    /// Number of results to display
    #[arg(short, long, default_value_t = 10)]
    count: u8,
}

#[derive(Debug)]
struct Args {
    ref_file_path: PathBuf,
    search_path: PathBuf,
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
        count,
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

fn main() {
    let input_args = InputArgs::parse();

    let args = validate_args(input_args);

    let now = std::time::Instant::now();
    let mut search_results = busca::run_search(&args.ref_file_path, &args.search_path).unwrap();
    println!("* COmpleted search in {} sec", now.elapsed().as_secs());

    search_results.sort_by(|a, b| b.perc_shared.partial_cmp(&a.perc_shared).unwrap());

    search_results.truncate(args.count.into());

    // println!("{}", &search_results);

    let file_matches = &search_results.to_string();
    let mut grid_options: Vec<_> = file_matches.split('\n').collect();

    // Remove the last new line
    grid_options.remove(grid_options.len() - 1);
    dbg!(&grid_options);

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
                    "{}{} {} |",
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
