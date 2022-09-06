use wordle_opt::{WordleOpt, Outcomes, bucket_label, info};
use wordle_opt::approx::{WordleOpt as ApproxWordleOpt, Outcomes as ApproxOutcomes};
use clap::Parser;

#[derive(Debug, Parser)]
enum Mode {
    Buckets {
        word: String,
        #[clap(long, short, default_value="5")]
        columns: usize
    },
    ApproxSolver,
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
    BestSecondConditional {
        first: String
    },
    BestSecondConditional2 {
        first: String
    },
    BestSecondConditional3 {
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

fn approx_bucket(word: &str, wordle_opt: &ApproxWordleOpt) -> (Vec<String>, f64) {
    let outcomes = ApproxOutcomes::new_uncompressed(&word, wordle_opt.answers_table());
    let buckets = outcomes.bucket();
    let info = info(&buckets);
    
    let mut entries = vec![];
    for (idx, count) in buckets.into_iter().enumerate() {
        if count > 0 {
            entries.push(format!("{}: {:>3}", idx, count));
        }
    }
    (entries, info)
}

fn string_to_outcome(s: &str) -> u8 {
    let mut rv = 0;
    for ch in s.chars() {
        rv *= 3;
        rv += match ch {
            '!' => 2,
            '?' => 1,
            '_' => 0,
            _ => panic!("Unknown character {ch}")
        }
    }
    rv
}

fn main() {
    let args = Args::parse();
    let words: Vec<_> = include_str!("words.txt").lines().collect();
    let answers: Vec<_> = include_str!("answers.txt").lines().collect();
    
    let wordle_opt = WordleOpt::new(&words, &answers);
    let approx_wordle_opt = ApproxWordleOpt::new(&words, &answers);
    match args.mode {
        Mode::Buckets { word, columns } => {
            let word = word.to_lowercase();
            let (entries, info) = bucket(&word, &wordle_opt);
            print_table(&entries, columns);
            println!("Info: {info}");
            // let outcomes = Outcomes::new_uncompressed(&word, wordle_opt.answers_table());
            // for (answer, outcome_2) in wordle_opt.answers().iter().zip(outcomes.outcomes) {
            //     let outcome_1 = answer.outcome(&word);
            //     println!("{}  {} {}; {} {}", answer.word, bucket_label(outcome_1 as usize), bucket_label(outcome_2 as usize), outcome_1, outcome_2);
            //     // assert_eq!(outcome_1, outcome_2);
            // }
        }
        Mode::Single { n } => {
            for word in wordle_opt.all_words().take(n) {
                println!("{} ({})", word.word, word.info);
            }
        },
        Mode::Pair => {
            if let Some((first, second, info)) = wordle_opt.best_two() {
                println!("{} {} ({})", first, second, info);
            } else {
                eprintln!("Unable to find a best pair?");
            }
        },
        Mode::PairsAboveThreshold { threshold } => {
            for (a, b, h) in wordle_opt.pairs_above_threshold(threshold) {
                println!("{a} {b} ({h})");
            }
        }
        Mode::PairsBelowThreshold { threshold } => {
            for (a, b, h) in wordle_opt.pairs_below_threshold(threshold) {
                println!("{a} {b} ({h})");
            }
        }
        Mode::Second { n, first } => {
            for word in wordle_opt.best_second_words(&first).into_iter().take(n) {
                println!("{} ({})", word.word, word.info);
            }
        },
        Mode::ApproxSolver => {
            let mut approx_wordle_opt = approx_wordle_opt.clone();
            loop {
                if approx_wordle_opt.answers().len() == 1 {
                    println!("Solution: {}", approx_wordle_opt.answers()[0].word);
                    break;
                } else {
                    let guess = approx_wordle_opt.all_words().next().unwrap();
                    for word in approx_wordle_opt.all_words().take(10) {
                        println!("  ({}; {})", word.word, word.info)
                    }
                    println!("{} ({})", guess.word, guess.info);
                    println!("{} {}", approx_wordle_opt.answers().len(), approx_wordle_opt.answers().iter().any(|a| a.word == "write"));
                    let (entries, info) = approx_bucket(guess.word, &approx_wordle_opt);
                    print_table(&entries, 5);
                    println!("Intermediate info: {info}");
                    let mut buf = String::new();
                    std::io::stdin().read_line(&mut buf).unwrap();
                    let hits = buf.trim().parse::<u8>().unwrap();
                    dbg!(hits);
                    approx_wordle_opt = approx_wordle_opt.pruned(guess.word, hits as usize);
                }
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
        Mode::BestSecondConditional { first } => {
            let results = wordle_opt.best_conditional_second(&first);
            
            for i in 0..6 {
                if let Some((word, count)) = results[i] {
                    println!("{i} hits: {} ({}; {count})", word.word, word.info);
                    
                } else {
                    println!("{i} hits: <impossible>");
                }
            }
            
        }
        Mode::BestSecondConditional2 { first } => {
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
                        println!("{} ({} bits, {} answers remain)", best_second.word, joint_info, pruned.answers().len());
                    }
                }
            }
        }
        Mode::BestSecondConditional3 { first } => {
            let mut clint_best = 0;
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
                    println!("{} ({:0.3} bits, {} answers remain)", best_second.word, joint_info, pruned.answers().len());
                    if best_second.word == "clint" {
                        clint_best += pruned.answers().len();
                    }
                }
            }
            println!("CLINT was best {clint_best} times ({:2.2}%)", (100 * clint_best) as f64 / wordle_opt.answers().len() as f64);
        }
        Mode::Third { n, first, second } => {
            for word in wordle_opt.best_third_word(&first, &second).into_iter().take(n) {
                println!("{} ({})", word.word, word.info);
            }
        },
        
    }
}
