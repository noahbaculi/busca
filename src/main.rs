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

fn main() {
    let input_args = InputArgs::parse();

    let args = validate_args(input_args);

    let mut search_results = busca::run_search(args.ref_file_path, args.search_path).unwrap();

    search_results.sort_by(|a, b| b.perc_shared.partial_cmp(&a.perc_shared).unwrap());

    search_results.truncate(args.count.into());

    use term_grid::{Cell, Direction, Filling, Grid, GridOptions};

    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(5),
        direction: Direction::LeftToRight,
    });

    grid.add(Cell::from("Path"));
    grid.add(Cell::from("Match"));

    for path_and_perc in search_results {
        grid.add(Cell::from(path_and_perc.path.display().to_string()));

        let perc_str = format!("{:.1}%", (path_and_perc.perc_shared * 100.0));
        grid.add(Cell::from(perc_str));
    }

    println!("{}", grid.fit_into_columns(2));
    // let grid_str = grid.fit_into_columns(2).to_string();
    // dbg!(grid_str);
}
