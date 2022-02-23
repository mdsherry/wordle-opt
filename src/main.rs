use wordle_opt::WordleOpt;

fn main() {
    let words: Vec<_> = include_str!("words.txt").lines().collect();
    let answers: Vec<_> = include_str!("answers.txt").lines().collect();
    
    let wordle_opt = WordleOpt::new(&words, &answers);
    // for word in wordle_opt.all_words().take(10) {
    //     println!("{:?}", word);
    // }
    let ts = std::time::Instant::now();
    println!("{:?}", wordle_opt.best_two());
    // println!("{:?}", wordle_opt.best_second_words("soare").len());
    println!("Time: {}s", ts.elapsed().as_secs());

}
