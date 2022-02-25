use wordle_opt::WordleOpt;
use clap::Parser;

#[derive(Debug, Parser)]
enum Mode {
    /// List the n best single starting words
    Single {
        /// Number of results to print out.
        #[clap(short, default_value="1")]
        n: usize
    },
    /// List the best pair of starting words
    Pair,
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

fn main() {
    let args = Args::parse();
    let words: Vec<_> = include_str!("words.txt").lines().collect();
    let answers: Vec<_> = include_str!("answers.txt").lines().collect();
    
    let wordle_opt = WordleOpt::new(&words, &answers);
    match args.mode {
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
        Mode::Second { n, first } => {
            for word in wordle_opt.best_second_words(&first).into_iter().take(n) {
                println!("{} ({})", word.word, word.info);
            }
        },
        Mode::Third { n, first, second } => {
            for word in wordle_opt.best_third_word(&first, &second).into_iter().take(n) {
                println!("{} ({})", word.word, word.info);
            }
        },
        
    }
}
