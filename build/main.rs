


pub mod output {
use serde::{Serialize, Deserialize};

use windjammer_runtime::io;

use windjammer_runtime::json;

use windjammer_runtime::time;

use crate::config::Config;

use crate::search::SearchResults;


pub const COLOR_RED: &'static str = "x1b[31m";
pub const COLOR_GREEN: &'static str = "x1b[32m";
pub const COLOR_BLUE: &'static str = "x1b[34m";
pub const COLOR_CYAN: &'static str = "x1b[36m";
pub const COLOR_RESET: &'static str = "x1b[0m";
pub const COLOR_BOLD: &'static str = "x1b[1m";

#[inline]
pub fn print_results(mut results: &SearchResults, mut config: &Config, mut duration: time::Duration) {
    if config.json {
        print_json(&results, duration)
    } else {
        if config.count_only {
            print_count(&results)
        } else {
            if config.files_with_matches {
                print_files_only(&results, &config)
            } else {
                print_default(&results, &config)
            }
        }
    }
}

#[inline]
pub fn print_json(mut results: &SearchResults, mut duration: time::Duration) {
    let output = json!(std::collections::HashMap::from([("matches", results.matches.iter().map(move |m| {
        json!(std::collections::HashMap::from([("file", m.file), ("line", m.line_number), ("column", m.column), ("text", m.line_text), ("match", m.match_text)]))
    }).collect::<Vec<_>>()), ("stats", std::collections::HashMap::from([("files_searched", results.files_searched), ("matches_found", results.total_matches), ("duration_ms", duration.as_millis())]))]));
    println!("{}", json::stringify_pretty(&output))
}

#[inline]
pub fn print_count(mut results: &SearchResults) {
    println!("{}", results.total_matches)
}

#[inline]
pub fn print_files_only(mut results: &SearchResults, mut config: &Config) {
    let mut files = std::collections::HashSet::new();
    for m in results.matches {
        files.insert(m.file);
    }
    for file in files {
        if config.use_color {
            println!("{}{}{}", COLOR_GREEN, file, COLOR_RESET)
        } else {
            println!("{}", file)
        }
    }
}

#[inline]
pub fn print_default(mut results: &SearchResults, mut config: &Config) {
    let mut by_file = std::collections::HashMap::new();
    for m in &results.matches {
        by_file.entry(m.file.clone()).or_insert(vec![]).push(m.clone());
    }
    for (file, matches) in by_file {
        if config.use_color {
            println!("{}{}{}{}", COLOR_BOLD, COLOR_GREEN, file, COLOR_RESET)
        } else {
            println!("{}", file)
        }
        for m in matches {
            print_match(&m, &config);
        }
        println!("");
    }
    if config.use_color {
        println!("{}Found {} matches in {} files (searched {} files){}", COLOR_CYAN, results.total_matches, by_file.len(), results.files_searched, COLOR_RESET)
    } else {
        println!("Found {} matches in {} files (searched {} files)", results.total_matches, by_file.len(), results.files_searched)
    }
}

#[inline]
pub fn print_match(mut m: &Match, mut config: &Config) {
    if !m.context_before.is_empty() {
        let start_line = m.line_number - m.context_before.len() as i64;
        for (i, context_line) in m.context_before.iter().enumerate() {
            let line_num = start_line + i as i64;
            print_context_line(line_num, &context_line, &config);
        }
    }
    let line_num_str = {
        if config.line_numbers {
            format!("{}:", m.line_number)
        } else {
            "".to_string()
        }
    };
    let highlighted_line = {
        if config.use_color {
            highlight_match(&m.line_text, &m.match_text, m.column)
        } else {
            m.line_text.clone()
        }
    };
    if config.use_color {
        println!("  {}{}{} {}", COLOR_BLUE, line_num_str, COLOR_RESET, highlighted_line)
    } else {
        println!("  {}{}", line_num_str, highlighted_line)
    }
    if !m.context_after.is_empty() {
        let start_line = m.line_number + 1;
        for (i, context_line) in m.context_after.iter().enumerate() {
            let line_num = start_line + i as i64;
            print_context_line(line_num, &context_line, &config);
        }
    }
}

#[inline]
pub fn print_context_line(mut line_num: i64, mut line: &str, mut config: &Config) {
    let line_num_str = {
        if config.line_numbers {
            format!("{}-", line_num)
        } else {
            "".to_string()
        }
    };
    if config.use_color {
        println!("  {}{}{} {}", COLOR_CYAN, line_num_str, COLOR_RESET, line)
    } else {
        println!("  {}{}", line_num_str, line)
    }
}

#[inline]
pub fn highlight_match(mut line: &str, mut match_text: &str, mut column: i64) -> String {
    let before = &&line[0..(column - 1) as usize];
    let after = &&line[(column - 1 + match_text.len() as i64) as usize..line.len()];
    format!("{}{}{}{}{}{}", before, COLOR_BOLD, COLOR_RED, match_text, COLOR_RESET, after)
}


}







pub mod gitignore {
use smallvec::{SmallVec, smallvec};

use windjammer_runtime::fs;

use windjammer_runtime::path;

use std::collections::HashSet;


#[derive(Clone)]
pub struct GitignoreRules {
    pub patterns: Vec<String>,
}

impl GitignoreRules {
#[inline]
pub fn new() -> Self {
        GitignoreRules { patterns: vec![] }
}
#[inline]
pub fn load_from_directory(&self, mut dir: String) -> Result<Self, String> {
        let gitignore_path = path::join(&dir, &".gitignore");
        if !fs::exists(&gitignore_path) {
            return Ok(GitignoreRules::new());
        }
        let contents = fs::read_to_string(&gitignore_path)?;
        let mut patterns: SmallVec<[_; 4]> = smallvec![];
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("#") {
                continue;
            }
            self.patterns.push(trimmed.to_string());
        }
        Ok(GitignoreRules { patterns })
}
#[inline]
pub fn is_ignored(&self, mut path: &str) -> bool {
        let name = path::file_name(&path).unwrap_or(path);
        let path_str = path.clone();
        for pattern in self.patterns.iter() {
            if self.matches_pattern(name.clone(), pattern.clone()) || self.matches_pattern(path_str.clone(), pattern.clone()) {
                return true;
            }
        }
        false
}
#[inline]
pub fn matches_pattern(&self, mut name: String, mut pattern: &str) -> bool {
        if name == pattern {
            return true;
        }
        if pattern.ends_with("/") {
            let dir_pattern = pattern.trim_end_matches("/");
            if name == dir_pattern {
                return true;
            }
        }
        if pattern.contains("*") {
            return self.wildcard_match(name, pattern);
        }
        if pattern.starts_with("*.") {
            let ext = pattern.trim_start_matches("*.");
            if name.ends_with(&format!(".{}", ext)) {
                return true;
            }
        }
        if name.contains(pattern) {
            return true;
        }
        false
}
#[inline]
pub fn wildcard_match(&self, mut name: String, mut pattern: &str) -> bool {
        let parts: Vec<String> = pattern.split('*').collect();
        if parts.is_empty() {
            return false;
        }
        if !parts[0].is_empty() && !name.starts_with(parts[0]) {
            return false;
        }
        if parts.len() > 1 {
            let last = &parts[parts.len() - 1];
            if !last.is_empty() && !name.ends_with(last) {
                return false;
            }
        }
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i == 0 {
                pos = part.len();
                continue;
            }
            match &name[pos..name.len()].find(part) {
                Some(idx) => {
                    pos = pos + idx + part.len();
                },
                _ => {
                    return false;
                },
            }
        }
        true
}
}

pub struct GitignoreCache {
    pub cache: std::collections::HashMap<String, GitignoreRules>,
}

impl GitignoreCache {
#[inline]
pub fn new() -> Self {
        GitignoreRules { patterns: vec![] }
}
#[inline]
pub fn get_rules(&mut self, mut dir: &str) -> GitignoreRules {
        match self.cache.get(&dir) {
            Some(rules) => {
                return rules.clone();
            },
        }
        let rules = GitignoreRules::load_from_directory(dir.clone()).unwrap_or_else(move |_| GitignoreRules::new());
        self.cache.insert(dir, rules.clone());
        rules
}
}


}



pub mod config {
use windjammer_runtime::regex_mod;

use crate::main::Args;


#[derive(Debug, Clone)]
pub struct Config {
    pub pattern: Regex,
    pub paths: Vec<String>,
    pub case_insensitive: bool,
    pub whole_word: bool,
    pub line_numbers: bool,
    pub count_only: bool,
    pub files_with_matches: bool,
    pub context_before: i64,
    pub context_after: i64,
    pub file_types: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_count: Option<i64>,
    pub threads: i64,
    pub json: bool,
    pub use_color: bool,
    pub search_hidden: bool,
    pub respect_ignore: bool,
}

#[inline]
pub fn from_args(mut args: Args) -> Result<Config, String> {
    let pattern_str = {
        if args.whole_word {
            let escaped = regex::escape(&args.pattern);
            format!("{}{}{}", "\\b", &escaped, "\\b")
        } else {
            args.pattern
        }
    };
    let pattern = {
        if args.case_insensitive {
            regex::compile_with_flags(&pattern_str, "i".to_string())?
        } else {
            regex::compile(&pattern_str)?
        }
    };
    let use_color = match args.color.as_str() {
        "always" => true,
        "never" => false,
        _ => std::io::is_terminal(std::io::stdout()),
    };
    Ok(Config { pattern, paths: args.paths, case_insensitive: args.case_insensitive, whole_word: args.whole_word, line_numbers: args.line_numbers, count_only: args.count_only, files_with_matches: args.files_with_matches, context_before: args.context_before, context_after: args.context_after, file_types: args.file_types, exclude_patterns: args.exclude, max_count: args.max_count, threads: args.threads, json: args.json, use_color, search_hidden: args.hidden, respect_ignore: !args.no_ignore })
}

#[inline]
pub fn get_file_extensions(mut file_type: String) -> Vec<String> {
    match file_type.as_str() {
        "rust" => vec!["rs"],
        "windjammer" | "wj" => vec!["wj"],
        "python" | "py" => vec!["py", "pyw"],
        "javascript" | "js" => vec!["js", "jsx", "mjs"],
        "typescript" | "ts" => vec!["ts", "tsx"],
        "go" => vec!["go"],
        "c" => vec!["c", "h"],
        "cpp" | "c++" => vec!["cpp", "cc", "cxx", "hpp", "hxx"],
        "java" => vec!["java"],
        "markdown" | "md" => vec!["md", "markdown"],
        "json" => vec!["json"],
        "yaml" | "yml" => vec!["yaml", "yml"],
        "toml" => vec!["toml"],
        "xml" => vec!["xml"],
        "html" => vec!["html", "htm"],
        "css" => vec!["css", "scss", "sass"],
        "sql" => vec!["sql"],
        "shell" | "sh" => vec!["sh", "bash", "zsh"],
        _ => vec![],
    }
}

#[inline]
pub fn matches_file_type(mut path: &str, mut file_types: &[String]) -> bool {
    if file_types.is_empty() {
        return true;
    }
    let ext = std::path::extension(path).unwrap_or("");
    for file_type in file_types {
        let extensions = get_file_extensions(file_type.clone());
        if extensions.contains(&ext.to_string()) {
            return true;
        }
    }
    false
}

#[inline]
pub fn should_exclude(mut path: &str, mut exclude_patterns: &[String]) -> bool {
    for pattern in exclude_patterns {
        if path.contains(pattern) {
            return true;
        }
    }
    false
}


}


pub mod matcher {
use smallvec::{SmallVec, smallvec};

use windjammer_runtime::regex_mod;

use crate::config::Config;

use crate::search::Match;


#[inline]
pub fn find_match(mut line: &str, mut line_num: i64, mut file: &str, mut config: &Config) -> Option<Match> {
    let captures = config.pattern.captures(line)?;
    let match_obj = captures.get(0)?;
    let match_text = match_obj.as_str();
    let column = (match_obj.start() + 1) as i64;
    Some(Match { file: file.clone(), line_number: line_num, column, line_text: line.to_string(), match_text: match_text.to_string(), context_before: vec![], context_after: vec![] })
}

#[inline]
pub fn find_all_matches(mut line: &str, mut line_num: i64, mut file: &str, mut config: &Config) -> Vec<Match> {
    let mut matches: SmallVec<[_; 4]> = smallvec![];
    for capture in config.pattern.captures_iter(line) {
        match capture.get(0) {
            Some(match_obj) => {
                let match_text = match_obj.as_str();
                let column = (match_obj.start() + 1) as i64;
                matches.push(Match { file: file.clone(), line_number: line_num, column, line_text: line.to_string(), match_text: match_text.to_string(), context_before: vec![], context_after: vec![] })
            },
        }
    }
    matches
}


}

pub mod search {
use smallvec::{SmallVec, smallvec};

use windjammer_runtime::fs;

use windjammer_runtime::io;

use windjammer_runtime::path;

use windjammer_runtime::sync;

use windjammer_runtime::thread;

use crate::config::Config;

use crate::walker;

use crate::matcher;


#[derive(Debug)]
pub struct SearchResults {
    pub matches: Vec<Match>,
    pub files_searched: i64,
    pub total_matches: i64,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub file: String,
    pub line_number: i64,
    pub column: i64,
    pub line_text: String,
    pub match_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[inline]
pub fn run(mut config: Config) -> Result<SearchResults, String> {
    let files = walker::collect_files(config.paths.clone(), &config)?;
    let matches = search_files_parallel(files.clone(), &config)?;
    let matches = match config.max_count {
        Some(max) => {
            matches.into_iter().take(max as usize).collect()
        },
        _ => {
            matches
        },
    };
    Ok(SearchResults { total_matches: matches.len() as i64, files_searched: files.len() as i64, matches })
}

#[inline]
pub fn search_files_parallel(mut files: Vec<String>, mut config: &Config) -> Result<Vec<Match>, String> {
    let num_threads = config.threads as usize;
    let chunk_size = (files.len() + num_threads - 1) / num_threads;
    let mut chunks: SmallVec<[_; 4]> = smallvec![];
    for i in 0..num_threads {
        let start = i * chunk_size;
        let end = std::cmp::min(start + chunk_size, files.len());
        if start < files.len() {
            chunks.push(&files[start..end].to_vec())
        }
    }
    let (tx, rx) = sync::channel();
    let mut handles: SmallVec<[_; 4]> = smallvec![];
    for chunk in chunks {
        let tx = tx.clone();
        let config = config.clone();
        let handle = {
            let _ = std::thread::spawn(move || {
                for file in chunk {
                    match search_file(file, &config.clone()) {
                        Ok(matches) => {
                            for m in matches {
                                tx.send(m).unwrap();
                            }
                        },
                        Err(_) => {
                        },
                    }
                }
            });
        };
        handles.push(handle);
    }
    drop(tx);
    let mut all_matches: SmallVec<[_; 4]> = smallvec![];
    loop {
        match rx.recv() {
            Ok(m) => {
                all_matches.push(m);
                match config.max_count {
                    Some(max) => {
                        if all_matches.len() >= max {
                            break;
                        }
                    },
                }
            },
            _ => {
                break;
            },
        }
    }
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(all_matches)
}

#[inline]
pub fn search_file(mut path: String, mut config: &Config) -> Result<Vec<Match>, String> {
    let contents = fs::read_to_string(&path)?;
    let all_lines: Vec<String> = contents.lines().map(move |s| s.to_string()).collect();
    let mut matches: SmallVec<[_; 4]> = smallvec![];
    for (line_num, line) in all_lines.iter().enumerate() {
        match matcher::find_match(line, (line_num + 1) as i64, &path, config) {
            Some(mut m) => {
                if config.context_before > 0 || config.context_after > 0 {
                    m = add_context(m, &all_lines, config.context_before, config.context_after);
                }
                matches.push(m);
                match config.max_count {
                    Some(max) => {
                        if matches.len() >= max as usize {
                            break;
                        }
                    },
                }
            },
        }
    }
    Ok(matches)
}

#[inline]
pub fn add_context(mut match_obj: Match, mut all_lines: &[String], mut lines_before: i64, mut lines_after: i64) -> Match {
    let line_idx = (match_obj.line_number - 1) as usize;
    let start_before = {
        if line_idx >= lines_before as usize {
            line_idx - lines_before as usize
        } else {
            0
        }
    };
    match_obj.context_before = &all_lines[start_before..line_idx].iter().map(move |s| s.clone()).collect();
    let start_after = line_idx + 1;
    let end_after = std::cmp::min(start_after + lines_after as usize, all_lines.len());
    match_obj.context_after = &all_lines[start_after..end_after].iter().map(move |s| s.clone()).collect();
    match_obj
}


}

pub mod walker {
use smallvec::{SmallVec, smallvec};

use windjammer_runtime::fs;

use windjammer_runtime::path;

use crate::config::Config;

use crate::gitignore::GitignoreCache;


#[inline]
pub fn collect_files(mut paths: Vec<String>, mut config: &Config) -> Result<Vec<String>, String> {
    let mut all_files: SmallVec<[_; 4]> = smallvec![];
    let mut gitignore_cache = GitignoreCache::new();
    for path in paths {
        let files = walk_path(path, &config, &mut gitignore_cache)?;
        all_files.extend(files);
    }
    Ok(all_files)
}

#[inline]
pub fn walk_path(mut path: String, mut config: &Config, mut gitignore_cache: &mut GitignoreCache) -> Result<Vec<String>, String> {
    let mut files: SmallVec<[_; 4]> = smallvec![];
    if !fs::exists(&path) {
        return Err(format!("Path does not exist: {}", path));
    }
    if fs::is_file(&path) {
        if should_include_file(&path, &config, &mut gitignore_cache) {
            files.push(path)
        }
        return Ok(files);
    }
    if fs::is_dir(&path) {
        walk_dir(path, &config, &mut files, &mut gitignore_cache)?
    }
    Ok(files)
}

#[inline]
pub fn walk_dir(mut dir: String, mut config: &Config, mut files: &mut [String], mut gitignore_cache: &mut GitignoreCache) -> Result<(), String> {
    let entries = fs::read_dir(&dir)?;
    let gitignore_rules = {
        if config.respect_ignore {
            Some(gitignore_cache.get_rules(&dir))
        } else {
            None
        }
    };
    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        if !config.search_hidden && file_name.starts_with(".") {
            continue;
        }
        if should_exclude(&path, &config.exclude_patterns) {
            continue;
        }
        if config.respect_ignore && is_ignored(&file_name) {
            continue;
        }
        if config.respect_ignore && gitignore_rules.is_some() {
            if gitignore_rules.as_ref().unwrap().is_ignored(&path) {
                continue;
            }
        }
        if entry.is_dir() {
            walk_dir(path, &config, &mut files, &mut gitignore_cache)?
        } else {
            if entry.is_file() {
                if should_include_file(&path, &config, &mut gitignore_cache) {
                    files.push(path)
                }
            }
        }
    }
    Ok(())
}

#[inline]
pub fn should_include_file(mut path: &str, mut config: &Config, mut gitignore_cache: &mut GitignoreCache) -> bool {
    if !matches_file_type(path, &config.file_types) {
        return false;
    }
    if config.respect_ignore {
        let dir = path::parent(&path).unwrap_or(".");
        let gitignore_rules = gitignore_cache.get_rules(&dir.to_string());
        if gitignore_rules.is_ignored(path) {
            return false;
        }
    }
    if is_likely_binary(&path) {
        return false;
    }
    true
}

#[inline]
pub fn is_ignored(mut name: &str) -> bool {
    let ignored_dirs = vec!["target", "node_modules", ".git", ".svn", ".hg", "dist", "build", "__pycache__", ".cache", ".venv", "venv"];
    ignored_dirs.contains(&name.as_str())
}

#[inline]
pub fn is_likely_binary(mut path: &str) -> bool {
    let binary_extensions = vec!["exe", "dll", "so", "dylib", "a", "o", "png", "jpg", "jpeg", "gif", "bmp", "ico", "pdf", "zip", "ta", "gz", "bz2", "xz", "mp3", "mp4", "avi", "mov", "mkv", "wasm", "class", "pyc"];
    let ext = path::extension(&path).unwrap_or("");
    binary_extensions.contains(&ext)
}


}

pub mod main {
use windjammer_runtime::cli;

use windjammer_runtime::fs;

use windjammer_runtime::io;

use windjammer_runtime::env;

use windjammer_runtime::time;

use windjammer_runtime::log_mod;

use crate::config;

use crate::search;

use crate::output;

use crate::gitignore;


#[derive(Debug)]
pub struct Args {
    pub pattern: String,
    pub paths: Vec<String>,
    pub case_insensitive: bool,
    pub whole_word: bool,
    pub line_numbers: bool,
    pub count_only: bool,
    pub files_with_matches: bool,
    pub context_before: i64,
    pub context_after: i64,
    pub file_types: Vec<String>,
    pub exclude: Vec<String>,
    pub max_count: Option<i64>,
    pub threads: i64,
    pub json: bool,
    pub color: String,
    pub hidden: bool,
    pub no_ignore: bool,
}

#[inline]
pub fn parse_args() -> Args {
    let mut app = cli::new(&"wjfind").version("0.1.0").author("Windjammer Team").about("Fast file search utility - like ripgrep, but in Windjammer!").arg(cli::arg("pattern".to_string()).help("Pattern to search for (regex)").required(true)).arg(cli::arg("paths".to_string()).help("Paths to search (default: current directory)").multiple(true).default_value(".")).arg(cli::flag(&"case-insensitive").short("i").help("Case-insensitive search")).arg(cli::flag(&"whole-word").short("w").help("Match whole words only")).arg(cli::flag(&"line-numbers").short("n").help("Show line numbers")).arg(cli::flag(&"count").short("c").help("Only show count of matches")).arg(cli::flag(&"files-with-matches").short("l").help("Only show files with matches")).arg(cli::option(&"context-before").short("B").help("Lines of context before match").default_value("0")).arg(cli::option(&"context-after").short("A").help("Lines of context after match").default_value("0")).arg(cli::option(&"context").short("C").help("Lines of context before and after match").default_value("0")).arg(cli::option(&"type").short("t").help("Filter by file type (rust, js, py, etc.)").multiple(true)).arg(cli::option(&"exclude").help("Exclude directories or files").multiple(true)).arg(cli::option(&"max-count").short("m").help("Maximum number of matches")).arg(cli::option(&"threads").short("j").help("Number of threads").default_value("0")).arg(cli::flag(&"json").help("Output results as JSON")).arg(cli::option(&"color").help("When to use colors (auto, always, never)").default_value("auto")).arg(cli::flag(&"hidden").help("Search hidden files and directories")).arg(cli::flag(&"no-ignore").help("Don't respect .gitignore files"));
    let matches = app.get_matches();
    let pattern = matches.value_of("pattern").unwrap();
    let paths = matches.values_of("paths").unwrap_or(vec!["."]);
    let case_insensitive = matches.is_present("case-insensitive");
    let whole_word = matches.is_present("whole-word");
    let line_numbers = matches.is_present("line-numbers");
    let count_only = matches.is_present("count");
    let files_with_matches = matches.is_present("files-with-matches");
    let context = matches.value_of("context").unwrap().parse::<i64>().unwrap_or(0);
    let context_before = {
        if context > 0 {
            context
        } else {
            matches.value_of("context-before").unwrap().parse::<i64>().unwrap_or(0)
        }
    };
    let context_after = {
        if context > 0 {
            context
        } else {
            matches.value_of("context-after").unwrap().parse::<i64>().unwrap_or(0)
        }
    };
    let file_types = matches.values_of("type").unwrap_or(vec![]);
    let exclude = matches.values_of("exclude").unwrap_or(vec![]);
    let max_count = matches.value_of("max-count").map(move |s| s.parse::<i64>().unwrap());
    let threads = matches.value_of("threads").unwrap().parse::<i64>().unwrap_or(0);
    let threads = {
        if threads == 0 {
            std.thread::available_parallelism().unwrap_or(4)
        } else {
            threads
        }
    };
    let json = matches.is_present("json");
    let color = matches.value_of("color").unwrap();
    let hidden = matches.is_present("hidden");
    let no_ignore = matches.is_present("no-ignore");
    Args { pattern, paths, case_insensitive, whole_word, line_numbers, count_only, files_with_matches, context_before, context_after, file_types, exclude, max_count, threads, json, color, hidden, no_ignore }
}


}


use windjammer_runtime::cli;

use windjammer_runtime::fs;

use windjammer_runtime::io;

use windjammer_runtime::env;

use windjammer_runtime::time;

use windjammer_runtime::log_mod;

use config;

use search;

use output;

use gitignore;


#[derive(Debug)]
struct Args {
    pattern: String,
    paths: Vec<String>,
    case_insensitive: bool,
    whole_word: bool,
    line_numbers: bool,
    count_only: bool,
    files_with_matches: bool,
    context_before: i64,
    context_after: i64,
    file_types: Vec<String>,
    exclude: Vec<String>,
    max_count: Option<i64>,
    threads: i64,
    json: bool,
    color: String,
    hidden: bool,
    no_ignore: bool,
}

fn main() {
    let args = parse_args();
    if env::var(&"WJFIND_LOG").is_ok() {
        log::init("debug".to_string())
    }
    let config = config::from_args(args).unwrap_or_else(move |e| {
        eprintln!("Error: {}", e);
        std::process::exit(1)
    });
    let start = time::now();
    let results = match search::run(config.clone()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1)
        },
    };
    let duration = time::now().duration_since(&start);
    output::print_results(&results, &config, duration);
    let exit_code = {
        if results.matches.is_empty() {
            1
        } else {
            0
        }
    };
    std::process::exit(exit_code)
}

fn parse_args() -> Args {
    let mut app = cli::new(&"wjfind").version("0.1.0").author("Windjammer Team").about("Fast file search utility - like ripgrep, but in Windjammer!").arg(cli::arg("pattern".to_string()).help("Pattern to search for (regex)").required(true)).arg(cli::arg("paths".to_string()).help("Paths to search (default: current directory)").multiple(true).default_value(".")).arg(cli::flag(&"case-insensitive").short("i").help("Case-insensitive search")).arg(cli::flag(&"whole-word").short("w").help("Match whole words only")).arg(cli::flag(&"line-numbers").short("n").help("Show line numbers")).arg(cli::flag(&"count").short("c").help("Only show count of matches")).arg(cli::flag(&"files-with-matches").short("l").help("Only show files with matches")).arg(cli::option(&"context-before").short("B").help("Lines of context before match").default_value("0")).arg(cli::option(&"context-after").short("A").help("Lines of context after match").default_value("0")).arg(cli::option(&"context").short("C").help("Lines of context before and after match").default_value("0")).arg(cli::option(&"type").short("t").help("Filter by file type (rust, js, py, etc.)").multiple(true)).arg(cli::option(&"exclude").help("Exclude directories or files").multiple(true)).arg(cli::option(&"max-count").short("m").help("Maximum number of matches")).arg(cli::option(&"threads").short("j").help("Number of threads").default_value("0")).arg(cli::flag(&"json").help("Output results as JSON")).arg(cli::option(&"color").help("When to use colors (auto, always, never)").default_value("auto")).arg(cli::flag(&"hidden").help("Search hidden files and directories")).arg(cli::flag(&"no-ignore").help("Don't respect .gitignore files"));
    let matches = app.get_matches();
    let pattern = matches.value_of("pattern").unwrap();
    let paths = matches.values_of("paths").unwrap_or(vec!["."]);
    let case_insensitive = matches.is_present("case-insensitive");
    let whole_word = matches.is_present("whole-word");
    let line_numbers = matches.is_present("line-numbers");
    let count_only = matches.is_present("count");
    let files_with_matches = matches.is_present("files-with-matches");
    let context = matches.value_of("context").unwrap().parse::<i64>().unwrap_or(0);
    let context_before = {
        if context > 0 {
            context
        } else {
            matches.value_of("context-before").unwrap().parse::<i64>().unwrap_or(0)
        }
    };
    let context_after = {
        if context > 0 {
            context
        } else {
            matches.value_of("context-after").unwrap().parse::<i64>().unwrap_or(0)
        }
    };
    let file_types = matches.values_of("type").unwrap_or(vec![]);
    let exclude = matches.values_of("exclude").unwrap_or(vec![]);
    let max_count = matches.value_of("max-count").map(move |s| s.parse::<i64>().unwrap());
    let threads = matches.value_of("threads").unwrap().parse::<i64>().unwrap_or(0);
    let threads = {
        if threads == 0 {
            std.thread::available_parallelism().unwrap_or(4)
        } else {
            threads
        }
    };
    let json = matches.is_present("json");
    let color = matches.value_of("color").unwrap();
    let hidden = matches.is_present("hidden");
    let no_ignore = matches.is_present("no-ignore");
    Args { pattern, paths, case_insensitive, whole_word, line_numbers, count_only, files_with_matches, context_before, context_after, file_types, exclude, max_count, threads, json, color, hidden, no_ignore }
}

