//! Command-line interface for the grep clone (v1 flag set).
//!
//! Cargo.toml needs:
//!   clap = { version = "4", features = ["derive"] }

use std::env::current_dir;
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader};
use std::iter::empty;
use std::{io, string};
use std::{fs, io::IsTerminal};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use regex::{Captures, Regex, regex};
use ignore::Walk;

const TEXT_EXTENSIONS: &[&str] = &["txt","lock", "md", "rs", "json", "csv", "log", "toml", "yaml", "xml", "html", "css", "js"];

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

    /// Match only whole words.
    #[arg(short = 'w', long)]
    pub word_regexp: bool,

    // // ---- Output ---------------------------------------------------------
    // /// Prefix each line with its line number.
    // #[arg(short = 'n', long)]
    // pub line_number: bool,

    /// Print only a count of matching lines per file.
    #[arg(short = 'c', long)]
    pub count: bool,

    // /// When to use colors.
    // #[arg(long, value_enum, default_value_t = ColorWhen::Auto, value_name = "WHEN")]
    // pub color: ColorWhen,

    // // ---- Context --------------------------------------------------------
    // /// Lines of context to print after each match.
    // #[arg(short = 'A', long, value_name = "NUM")]
    // pub after_context: Option<usize>,

    // /// Lines of context to print before each match.
    // #[arg(short = 'B', long, value_name = "NUM")]
    // pub before_context: Option<usize>,

    // /// Lines of context before and after each match (-A / -B override it).
    // #[arg(short = 'C', long, value_name = "NUM")]
    // pub context: Option<usize>,

    // ---- Files ----------------------------------------------------------
    
    /// Search directories recursively (skips hidden files).
    #[arg(short = 'r', long)]
    pub recursive: bool,
}

impl Cli {
    /// Lines of leading context, with -B taking precedence over -C.
    /// Context is meaningless with -c, so it resolves to 0 there.
    // pub fn before(&self) -> usize {
    //     if self.count {
    //         return 0;
    //     }
    //     self.before_context.or(self.context).unwrap_or(0)
    // }

    // /// Lines of trailing context, with -A taking precedence over -C.
    // pub fn after(&self) -> usize {
    //     if self.count {
    //         return 0;
    //     }
    //     self.after_context.or(self.context).unwrap_or(0)
    // }

    // /// Resolve --color against whether stdout is a terminal.
    // pub fn use_color(&self) -> bool {
    //     match self.color {
    //         ColorWhen::Always => true,
    //         ColorWhen::Never => false,
    //         ColorWhen::Auto => std::io::stdout().is_terminal(),
    //     }
    // }

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

        let final_bound = if self.ignore_case{
            format!(r"(?i){}", word_pattern)
        }else{
            word_pattern
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

    fn configure_path(&mut self, paths:&Vec<PathBuf>)-> io::Result<()>{
        let mut is_dir = false;
        for path in paths{
            let pth = if path.to_str().unwrap() == "." {
                current_dir()?
            }else{
                PathBuf::from(path)
            };

            if pth.is_dir(){
                is_dir= true;
                break;
            }
        }

        if is_dir{
            let final_paths = scan_text_files_by_extension(paths[0].clone());

            self.paths = final_paths;
            
        }else{
            self.paths = paths.to_vec();
        }

        Ok(())
    }
}

fn main()-> io::Result<()>{
    let mut cli = Cli::parse();

    let pattern = cli.pattern.clone().unwrap_or_else(|| String::from(""));

    
    cli.configure_path(&cli.paths.clone())?;
    
    let paths = &cli.paths;


    for path in paths{
        let mut title_condition = false;
        
        let mut line_count = 0;
        let mut count_entry = 0;

        let reader: BufReader<File> = cli.clone().read_file(path.clone());

        for line in reader.lines(){
            if let Ok(line) = line{
                line_count += 1;

                let (re, is_match) = cli.is_match(&line, &pattern);

                if cli.count && is_match{
                    count_entry+=1;
                }else{
                    if cli.invert_match && !is_match{
                        if !title_condition{
                            println!("\n\x1b[38;2;0;128;0m{:?}\x1b[0m\n", path);
                            title_condition = true;
                        }
                        println!("line \x1b[38;2;0;128;0m{}\x1b[0m: {}", line_count, line);
                    }else if is_match && !cli.invert_match{
                        if !title_condition{
                            println!("\n\x1b[38;2;0;128;0m{:?}\x1b[0m\n", path);
                            title_condition = true;
                        }

                        let colored_sentence = re.replace_all(&line, |caps: &Captures| {
                            format!("\x1b[38;2;255;0;0m{}\x1b[0m", &caps[0])
                        });
                        
                        println!("line \x1b[38;2;0;128;0m{}\x1b[0m: {}", line_count, colored_sentence);
                    }
                }

            }
        }

        if cli.count{
            println!("Count \x1b[38;2;0;128;0m{}\x1b[0m", count_entry);
        }
    }

    Ok(())

}

fn is_text_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext_str| TEXT_EXTENSIONS.contains(&ext_str.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn scan_text_files_by_extension<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    let mut vec_path = vec![];

    for entry in Walk::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() && is_text_extension(path) {
            vec_path.push(path.to_path_buf());
        }
    }

    vec_path
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