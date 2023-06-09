use chrono::{Local, TimeZone};
use clap::Parser;
use git2::Repository;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn main() {
    let cli = Cli::parse();
    let project_dir = cli.directory;
    if let Err(e) = run(project_dir) {
        println!("Error: {}", e);
    }
}

/// A program to see when you last updated your specs in your Gemfile.lock.
#[derive(Parser)]
struct Cli {
    /// The directory of the bundler project you want to check.
    directory: String,
}

fn run(project_dir: String) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = Path::new(&project_dir);
    let gemfile_lock = "Gemfile.lock";
    let gemfile_lock_path = Path::new(gemfile_lock);
    let git_dir = ".git";
    let git_path = project_path.join(git_dir);

    let (spec_lines, gemfile_lock_lines) = get_spec_lines(project_path.join(gemfile_lock));

    let repo = Repository::open(git_path).unwrap();
    // Instead of `None` you can also pass a `git2::BlameOptions` object.
    let blame = repo.blame_file(gemfile_lock_path, None).unwrap();
    let hunks = blame.iter().collect::<Vec<_>>();
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for hunk in hunks {
        let seconds = hunk.final_signature().when().seconds();
        let formatted_time = format_seconds(seconds);

        map.entry(formatted_time)
            .and_modify(|lines| {
                let start_line = hunk.final_start_line() - 1 as usize;
                let end_line = start_line + hunk.lines_in_hunk() as usize;

                let mut current_line = start_line;
                while current_line < end_line {
                    if spec_lines.contains(&current_line) {
                        let line = gemfile_lock_lines[current_line].clone();
                        lines.push(line);
                    }
                    current_line += 1;
                }
            })
            .or_insert(vec![]);
    }

    for (time, lines) in &mut map {
        if lines.len() > 0 {
            println!("Updated {}:", time);
            for line in lines {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

fn format_seconds(seconds: i64) -> String {
    let local_time = Local.timestamp_opt(seconds, 0).unwrap();
    local_time.format("%Y-%m-%d").to_string()
}

fn get_spec_lines(path: PathBuf) -> (HashSet<usize>, Vec<String>) {
    let mut set = HashSet::new();
    let mut lines = Vec::new();
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let re = Regex::new(r"\(\d+\.\d+\.\d+\.?\d?\)").unwrap();

    for (i, line) in reader.lines().enumerate() {
        if let Ok(line) = line {
            if re.is_match(line.as_str()) {
                set.insert(i);
            }
            lines.push(line);
        }
    }

    (set, lines)
}
