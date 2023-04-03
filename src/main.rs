use chrono::{Local, TimeZone};
use git2::Repository;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        panic!("Please provide a project directory.");
    }

    let project_dir = args[1].clone();
    if let Err(e) = run(&project_dir) {
        println!("Error: {}", e);
    }
}

fn run(project_dir: &String) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = Path::new(project_dir);
    let gemfile_lock = "Gemfile.lock";
    let gemfile_lock_path = Path::new(gemfile_lock);
    let git_dir = ".git";
    let git_path = project_path.join(git_dir);

    let gem_file_lock_lines = read_file_lines(project_path.join(gemfile_lock));

    let repo = Repository::open(git_path).unwrap();
    // Instead of `None` you can also pass a `git2::BlameOptions` object.
    let blame = repo.blame_file(gemfile_lock_path, None).unwrap();
    let mut hunks = blame.iter().collect::<Vec<_>>();
    hunks.sort_by(|a, b| a.final_signature().when().cmp(&b.final_signature().when()));

    for hunk in hunks {
        let commit_id = hunk.final_commit_id();
        let seconds = hunk.final_signature().when().seconds();
        let formatted_time = format_seconds(seconds);
        println!("{} -- {}", commit_id, formatted_time);

        let start_line = hunk.final_start_line() - 1 as usize;
        let end_line = start_line + hunk.lines_in_hunk() as usize;
        println!("Lines {} to {}", start_line + 1, end_line + 1);
        let hunk_lines = gem_file_lock_lines[start_line..end_line].join("\n");
        println!("{}", hunk_lines);

        println!();
    }
    Ok(())
}

fn format_seconds(seconds: i64) -> String {
    let local_time = Local.timestamp_opt(seconds, 0).unwrap();
    local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string()
}

fn read_file_lines(path: PathBuf) -> Vec<String> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
    lines
}
