//! Command-line interface for the grep clone (v1 flag set).
//!
//! Cargo.toml needs:
//!   clap = { version = "4", features = ["derive"] }

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::empty;
use std::{io, string};
use std::{fs, io::IsTerminal};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use regex::{Captures, Regex, regex};

/// When to colorize output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ColorWhen {
    /// Color only when stdout is a terminal.
    #[default]
    Auto,
    Always,
    Never,
}

/// A small grep clone.
#[derive(Debug, Parser,Clone)]
#[command(name = "rgrep", version, about, arg_required_else_help = true)]
pub struct Cli {
    /// Pattern to search for (regex by default, literal with -F).
    #[arg(value_name = "PATTERN")]
    pattern: Option<String>,

    /// Files or directories to search. Reads stdin if none are given. file
    /// (or searches "." when -r is set).
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    // ---- Matching -------------------------------------------------------
    /// Case-insensitive matching.
    #[arg(short = 'i', long)]
    ignore_case: bool,

    /// Select non-matching lines.
    #[arg(short = 'v', long)]
    pub invert_match: bool,

    /// Treat PATTERN as a literal string, not a regex.
    #[arg(short = 'F', long)]
    pub fixed_strings: bool,

    /// Match only whole words.
    #[arg(short = 'w', long)]
    pub word_regexp: bool,

    // ---- Output ---------------------------------------------------------
    /// Prefix each line with its line number.
    #[arg(short = 'n', long)]
    pub line_number: bool,

    /// Print only a count of matching lines per file.
    #[arg(short = 'c', long)]
    pub count: bool,

    /// When to use colors.
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, value_name = "WHEN")]
    pub color: ColorWhen,

    // ---- Context --------------------------------------------------------
    /// Lines of context to print after each match.
    #[arg(short = 'A', long, value_name = "NUM")]
    pub after_context: Option<usize>,

    /// Lines of context to print before each match.
    #[arg(short = 'B', long, value_name = "NUM")]
    pub before_context: Option<usize>,

    /// Lines of context before and after each match (-A / -B override it).
    #[arg(short = 'C', long, value_name = "NUM")]
    pub context: Option<usize>,

    // ---- Files ----------------------------------------------------------
    /// Search directories recursively (skips hidden files).
    #[arg(short = 'r', long)]
    pub recursive: bool,
}

impl Cli {
    /// Lines of leading context, with -B taking precedence over -C.
    /// Context is meaningless with -c, so it resolves to 0 there.
    pub fn before(&self) -> usize {
        if self.count {
            return 0;
        }
        self.before_context.or(self.context).unwrap_or(0)
    }

    /// Lines of trailing context, with -A taking precedence over -C.
    pub fn after(&self) -> usize {
        if self.count {
            return 0;
        }
        self.after_context.or(self.context).unwrap_or(0)
    }

    /// Resolve --color against whether stdout is a terminal.
    pub fn use_color(&self) -> bool {
        match self.color {
            ColorWhen::Always => true,
            ColorWhen::Never => false,
            ColorWhen::Auto => std::io::stdout().is_terminal(),
        }
    }

    /// Paths to search. An empty result means "read stdin".
    pub fn search_paths(&self) -> Vec<PathBuf> {
        if self.paths.is_empty() && self.recursive {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        }
    }

    /// Show filename prefixes when searching more than one file or recursing.
    pub fn show_filename(&self) -> bool {
        self.recursive || self.paths.len() > 1
    }

    fn read_file(&self, input: PathBuf)->BufReader<File>{
        let file = File::open(input).unwrap_or_else(|e| {
            eprintln!("Error opening file: {}", e);
            std::process::exit(1);
        });

        let reader = io::BufReader::new(file);

        return reader;
    }

    fn is_match(&self, line: &str, pattern: &str) -> (Regex, bool) {

        let word_pattern = if self.word_regexp{
            format!(r"\b{}\b", pattern)
        }else{
            format!("{}", pattern)
        };

        let case_insensitive = if self.ignore_case{
            format!(r"(?i){}", word_pattern)
        }else{
            word_pattern
        };

        let final_bound = if self.invert_match{
            format!(r"\(?!{})\w+", case_insensitive)
        }else{
            case_insensitive
        };


        // if self.ignore_case && self.word_regexp==false{
        //     pttn = format!(r"(?i)\b{}", pattern);
        // }else if self.ignore_case && self.word_regexp{
        //     pttn = format!(r"(?i)\b{}\b", pattern);
        // }else if self.word_regexp{
        //     pttn = format!(r"\b{}\b", pattern);
        // }else{}

        let re = Regex::new(&final_bound).unwrap();

        return (re.clone(), re.is_match(line).clone());
    }
}

fn main()-> io::Result<()>{
    let cli = Cli::parse();

    let pattern = cli.pattern.clone().unwrap_or_else(|| String::from(""));

    let paths = &cli.paths;

    for path in paths{
        let mut line_count = 0;

        let reader: BufReader<File> = cli.clone().read_file(path.clone());

        for line in reader.lines(){
            if let Ok(line) = line{
                line_count += 1;

                let (re, is_match) = cli.is_match(&line, &pattern);

                if is_match{
                    let colored_sentence = re.replace_all(&line, |caps: &Captures| {
                        format!("\x1b[38;2;255;0;0m{}\x1b[0m", &caps[0])
                    });
                    
                    println!("line \x1b[38;2;0;128;0m{}\x1b[0m: {}", line_count, colored_sentence);
                }
            }
        }

    }

    Ok(())

}


// for line in reader.lines(){
        //     if let Ok(line) = line{
        //         line_count+=1;
        
        //        let pttn = format!(r"\b{}\b", pattern);

        //         let re = Regex::new(&pttn).unwrap();

        //         if re.is_match(&line){
        //             println!("line {}:{}", line_count, line);
        //         }

        //     } else {
        //         eprintln!("Error reading line");
        //     }
        // }

        // Ok(())