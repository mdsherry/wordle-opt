use std::path::PathBuf;

use wordle_opt::{WordleOpt, Outcomes, bucket_label, info};
use clap::Parser;

#[derive(Debug, Parser)]
enum Mode {
    /// List the possible hints the word might receive in response, and how many answers would lead to that hint
    Buckets {
        word: String,
        #[clap(long, short, default_value="5")]
        columns: usize
    },
    /// Interactive solver mode
    Solver,
    /// List the n best single starting words
    Single {
        /// Number of results to print out.
        #[clap(short, default_value="1")]
        n: usize
    },
    /// List the best pair of starting words
    Pair,
    /// List all pairs above an entropy threshold
    PairsAboveThreshold {
        threshold: f64
    },
    /// List all pairs below an entropy threshold
    PairsBelowThreshold {
        threshold: f64
    },
    /// List the most informative second words to use based on the sum of the number of yellow and green squares in the hint
    BestSecondHits {
        first: String
    },
    /// List the most informative second words to use based on the number of yellow squares and green squares in the hint
    BestSecondCounts {
        first: String
    },
    /// List the most informative second words to use based on the exact hint
    BestSecondPrecise {
        first: String
    },
    /// Given a starting word, what second word will be useful in the grestest number of circumstances?
    Second{
        /// Number of results to print out.
        #[clap(short, default_value="1")]
        n: usize,
        /// First starting word
        first: String
    },
    /// Given a starting pair of words, what third word will be useful in the grestest number of circumstances?
    Third{
        /// Number of results to print out.
        #[clap(short, default_value="1")]
        n: usize,
        /// First starting word
        first: String,
        /// Second starting word
        second: String
    }
}

#[derive(Debug, Parser)]
struct Args {
    /// Path to alternate list of answers; if not provided, it will default to the standard Wordle list
    #[clap(long)]
    answers: Option<PathBuf>,
    /// Path to alternate list of additional guessable words; if not provided, it will default to the standard Wordle list
    #[clap(long)]
    words: Option<PathBuf>,
    /// Number of decimals to use when printing out entropy calculations. Default: 3
    #[clap(long, default_value="3")]
    precision: usize,
    #[clap(subcommand)]
    mode: Mode
}

fn print_table(entries: &[String], columns: usize) {
    let max_width = entries.iter().map(|s| s.len()).max().unwrap_or_default();
    for chunk in entries.chunks(columns) {
        for col in chunk {
            print!("{col:max_width$}    ");
        }
        println!();
    }
}

pub fn quasiinfo<'a>(buckets: &[u16]) -> f64 {
    let total = buckets.iter().copied().sum::<u16>() as f64;
    let mut h = 0.;
    for &bucket in buckets {
        if bucket > 0 {
            let p = (bucket as f64) / total;
            
            h += p * (1. - p)
        }
    }
    h
}


fn bucket(word: &str, wordle_opt: &WordleOpt) -> (Vec<String>, f64) {
    let outcomes = Outcomes::new_uncompressed(&word, wordle_opt.answers_table());
    let buckets = outcomes.bucket();
    let info = info(&buckets);
    
    let mut entries = vec![];
    for (idx, count) in buckets.into_iter().enumerate() {
        if count > 0 {
            entries.push(format!("{}: {:>3}", bucket_label(idx), count));
        }
    }
    (entries, info)
}

fn string_to_outcome(s: &str) -> u8 {
    let mut rv = 0;
    for ch in s.chars() {
        rv *= 3;
        rv += match ch {
            'Y' | 'y' | '!' => 2,
            'G' | 'g' | '?' => 1,
            '.' | '_' => 0,
            _ => panic!("Unknown character {ch}")
        }
    }
    rv
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let words: Vec<_> = if let Some(word_path) = args.words {
        let contents = std::fs::read_to_string(word_path)?;
        let contents = Box::leak(contents.into_boxed_str());
        contents.lines().collect()
    } else {
         include_str!("words.txt").lines().collect()
    };
    let answers: Vec<_> = if let Some(answers_path) = args.answers {
        let contents = std::fs::read_to_string(answers_path)?;
        let contents = Box::leak(contents.into_boxed_str());
        contents.lines().collect()
    } else {
         include_str!("answers.txt").lines().collect()
    };
    
    let precision = args.precision;
    let wordle_opt = WordleOpt::new(&words, &answers);
    
    match args.mode {
        Mode::Buckets { word, columns } => {
            let word = word.to_lowercase();
            let (entries, info) = bucket(&word, &wordle_opt);
            print_table(&entries, columns);
            println!("Info: {info}");

        }
        Mode::Single { n } => {
            for word in wordle_opt.all_words().take(n) {
                println!("{} ({:0.precision$})", word.word, word.info);
            }
        },
        Mode::Pair => {
            if let Some((first, second, info)) = wordle_opt.best_two() {
                println!("{} {} ({:0.precision$})", first, second, info);
            } else {
                eprintln!("Unable to find a best pair?");
            }
        },
        Mode::PairsAboveThreshold { threshold } => {
            for (a, b, h) in wordle_opt.pairs_above_threshold(threshold) {
                println!("{a} {b} ({h:0.precision$})");
            }
        }
        Mode::PairsBelowThreshold { threshold } => {
            for (a, b, h) in wordle_opt.pairs_below_threshold(threshold) {
                println!("{a} {b} ({h:0.precision$})");
            }
        }
        Mode::Second { n, first } => {
            for word in wordle_opt.best_second_words(&first).into_iter().take(n) {
                println!("{} ({:0.precision$})", word.word, word.info);
            }
        }
        Mode::Solver => {
            let mut wordle_opt = wordle_opt.clone();
            loop {
                if wordle_opt.answers().len() == 1 {
                    println!("Solution: {}", wordle_opt.answers()[0].word);
                    break;
                } else {
                    let guess = wordle_opt.all_words().next().unwrap();
                    println!("{} ({})", guess.word, guess.info);
                    println!("{} {}", wordle_opt.answers().len(), wordle_opt.answers().iter().any(|a| a.word == "brass"));
                    let mut buf = String::new();
                    std::io::stdin().read_line(&mut buf).unwrap();
                    let hits = string_to_outcome(buf.trim());
                    
                    wordle_opt = wordle_opt.pruned_exact(guess.word, hits);
                }
            }
        }
        Mode::BestSecondHits { first } => {
            for hits in 0..6 {
                let pruned = wordle_opt.pruned(&first, hits);
                if pruned.answers().is_empty() {
                    continue;
                }
                print!("{hits} hits: ");
                if pruned.answers().len() == 1 {
                    println!("{} (only answer left)", pruned.answers()[0].word)
                } else {
                    let first_outcome = Outcomes::from_answers_table(&first, pruned.answers_table());
                    let first_info = info(&first_outcome.bucket());
                    let best_second = pruned.best_second_words(&first)[0];
                    let joint_info = best_second.info + first_info;
                    println!("{} ({:0.precision$} bits, {} answers remain)", best_second.word, joint_info, pruned.answers().len());
                
                }
            }
            
        }
        Mode::BestSecondCounts { first } => {
            for yellow in 0..6 {
                for green in 0..(6 - yellow) {
                    let pruned = wordle_opt.pruned_2(&first, yellow, green);
                    if pruned.answers().is_empty() {
                        continue;
                    }
                    print!("{yellow}Y, {green}G: ");
                    if pruned.answers().len() == 1 {
                        println!("{} (only answer left)", pruned.answers()[0].word)
                    } else {
                        let first_outcome = Outcomes::from_answers_table(&first, pruned.answers_table());
                        let first_info = info(&first_outcome.bucket());
                        let best_second = pruned.best_second_words(&first)[0];
                        let joint_info = best_second.info + first_info;
                        println!("{} ({:0.precision$} bits, {} answers remain)", best_second.word, joint_info, pruned.answers().len());
                    }
                }
            }
        }
        Mode::BestSecondPrecise { first } => {
            for i in 0..243 {
                let pruned = wordle_opt.pruned_exact(&first, i);
                if pruned.answers().is_empty() {
                    continue;
                }
                print!("{}: ", bucket_label(i as usize));
                if pruned.answers().len() == 1 {
                    println!("{} (only answer left)", pruned.answers()[0].word)
                } else {
                    let first_outcome = Outcomes::from_answers_table(&first, pruned.answers_table());
                    let first_info = info(&first_outcome.bucket());
                    let best_second = pruned.best_second_words(&first)[0];
                    let joint_info = best_second.info + first_info;
                    println!("{} ({:0.precision$} bits, {} answers remain)", best_second.word, joint_info, pruned.answers().len());
                }
            }
            
        }
        Mode::Third { n, first, second } => {
            for word in wordle_opt.best_third_word(&first, &second).into_iter().take(n) {
                println!("{} ({:0.precision$})", word.word, word.info);
            }
        },
        
    }
    Ok(())
}
