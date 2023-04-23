use clap::Parser;
use std::{env, path::PathBuf};

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
    search: busca::Search,
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

    let search_kind: busca::SearchKind;
    if search_path.is_file() {
        search_kind = busca::SearchKind::File;
    } else if search_path.is_dir() {
        search_kind = busca::SearchKind::Directory;
    } else {
        panic!(
            "The search path '{}' could not be found",
            search_path.display()
        );
    }
    let search = busca::Search {
        path: search_path,
        kind: search_kind,
    };

    let mut count = input_args.count;
    if (count == 0) | (count > 200) {
        count = 10
    }

    Args {
        ref_file_path: input_args.ref_file_path,
        search,
        count,
    }
}

fn main() {
    let input_args = InputArgs::parse();

    let args = validate_args(input_args);
    dbg!(&args.count);

    let _result = busca::run_search(args.ref_file_path, args.search).unwrap();
    // println!("{}", _result);
}
