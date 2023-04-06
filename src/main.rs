use chrono::{Local, TimeZone};
use git2::Repository;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn main() {
    let project_dir = parse();
    if let Err(e) = run(project_dir) {
        println!("Error: {}", e);
    }
}

fn parse() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        panic!("Please provide a project directory.");
    }

    let project_dir = args[1].clone();
    project_dir
}

#[derive(Debug)]
struct Specs {
    start_line: usize,
    end_line: usize,
}

impl Specs {
    fn new(start_line: usize, end_line: usize) -> Self {
        Specs {
            start_line,
            end_line,
        }
    }

    fn overlaps(&self, start_line: usize, end_line: usize) -> bool {
        if start_line > self.end_line {
            return false;
        }
        if end_line < self.start_line {
            return false;
        }
        true
    }

    fn hunk_limits(&self, start_line: usize, end_line: usize) -> (usize, usize) {
        let start = std::cmp::max(self.start_line, start_line);
        let end = std::cmp::min(self.end_line, end_line);
        (start, end)
    }
}

struct AllSpecs {
    specs: Vec<Specs>,
}

impl AllSpecs {
    fn from_lines(lines: &Vec<String>) -> Self {
        let mut specs = Vec::new();
        let mut start_line = None;
        let mut end_line = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.contains("specs:") {
                start_line = Some(i + 1);
            }
            if line.is_empty() {
                if let Some(start) = start_line {
                    end_line = i;
                    specs.push(Specs::new(start, end_line));
                    start_line = None;
                }
            }
        }
        AllSpecs { specs }
    }

    fn overlaps(&self, start_line: usize, end_line: usize) -> bool {
        self.specs
            .iter()
            .any(|spec| spec.overlaps(start_line, end_line))
    }

    fn hunk_limits(&self, start_line: usize, end_line: usize) -> (usize, usize) {
        let mut start = 0;
        let mut end = 0;
        for spec in self.specs.iter() {
            if spec.overlaps(start_line, end_line) {
                let (s, e) = spec.hunk_limits(start_line, end_line);
                start = s;
                end = e;
            }
        }
        (start, end)
    }
}

fn run(project_dir: String) -> Result<(), Box<dyn std::error::Error>> {
    let project_path = Path::new(&project_dir);
    let gemfile_lock = "Gemfile.lock";
    let gemfile_lock_path = Path::new(gemfile_lock);
    let git_dir = ".git";
    let git_path = project_path.join(git_dir);

    let gemfile_lock_lines = read_file_lines(project_path.join(gemfile_lock));
    let all_specs = AllSpecs::from_lines(&gemfile_lock_lines);

    let repo = Repository::open(git_path).unwrap();
    // Instead of `None` you can also pass a `git2::BlameOptions` object.
    let blame = repo.blame_file(gemfile_lock_path, None).unwrap();
    let mut hunks = blame.iter().collect::<Vec<_>>();
    hunks.sort_by(|a, b| a.final_signature().when().cmp(&b.final_signature().when()));

    for hunk in hunks {
        let seconds = hunk.final_signature().when().seconds();
        let formatted_time = format_seconds(seconds);

        let start_line = hunk.final_start_line() - 1 as usize;
        let end_line = start_line + hunk.lines_in_hunk() as usize;

        if all_specs.overlaps(start_line, end_line) {
            println!("Updated {} on {}.", time_passed(seconds), formatted_time);
            let (start, end) = all_specs.hunk_limits(start_line, end_line);
            println!("Lines {} through {}.", start + 1, end);
            let hunk_lines = gemfile_lock_lines[start..end].join("\n");
            println!("{}", hunk_lines);
            println!();
        }
    }

    Ok(())
}

fn time_passed(seconds: i64) -> String {
    let now = Local::now();
    let then = Local.timestamp_opt(seconds, 0).unwrap();
    let duration = now.signed_duration_since(then);
    let days = duration.num_days();
    let hours = duration.num_hours();
    let minutes = duration.num_minutes();

    if days > 0 {
        return format!("{} days ago", days);
    }
    if hours > 0 {
        return format!("{} hours ago", hours);
    }
    if minutes > 0 {
        return format!("{} minutes ago", minutes);
    }

    "just now".to_string()
}

fn format_seconds(seconds: i64) -> String {
    let local_time = Local.timestamp_opt(seconds, 0).unwrap();
    local_time.format("%B %d, %Y").to_string()
}

fn read_file_lines(path: PathBuf) -> Vec<String> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
    lines
}
