use std::sync::atomic::AtomicU32;

use atomic_float::AtomicF64;
use ordered_float::OrderedFloat;
use rayon::prelude::*;
use rustc_hash::FxHashMap;

const BUCKETS_SIZE: usize = 3 * 81;
type Buckets = [u16; BUCKETS_SIZE];

fn info<'a>(buckets: impl IntoIterator<Item=&'a u16>) -> f64 {
    let mut total = 0.;
    let mut h = 0.;
    for &bucket in buckets {
        if bucket > 0 {
            // let p = (bucket as f64) / total;
            let p = bucket as f64;
            total += p;
            h += p * p.log2();
        }
    }
    h = h / total - total.log2();
    -h
}

fn info_two(outcomes1: &Outcomes, outcomes2: &Outcomes, buckets: &mut FxHashMap<usize, u16>, idxs: &mut Vec<usize>) -> f64 {
    // let mut buckets2 = [0; BUCKETS_SIZE * BUCKETS_SIZE];
    // let mut buckets2 = vec![0; BUCKETS_SIZE * BUCKETS_SIZE];
    buckets.clear();
    // idxs.clear();
    let mut idxs = vec![];
    idxs.extend(outcomes1.outcomes.iter().zip(&outcomes2.outcomes).map(|(&a, &b)| a as usize * BUCKETS_SIZE + b as usize));
    // for (outcome1, outcome2) in outcomes1.outcomes.iter().zip(&outcomes2.outcomes) {
    for idx in idxs.iter().copied() {
        // let idx = *outcome1 as usize * BUCKETS_SIZE + *outcome2 as usize;
        // buckets2[idx] += 1;
        *buckets.entry(idx).or_default() += 1;
    }
    info(buckets.values())

}

#[derive(Debug)]
pub struct AugmentedWord<'a> {
    pub word: &'a str,
    pub info: f64
}

struct AugmentedAnswer<'a> {
    word: &'a str,
    bitmask: u32
}

impl<'a> AugmentedAnswer<'a> {
    fn new(word: &'a str) -> Self {
        let mut bitmask = 0;
        for ch in word.chars() {
            assert!(ch.is_ascii_lowercase());
            bitmask |= 1 << (ch as u8  - b'a');
        }
        Self { word, bitmask }
    }
}

#[derive(Clone)]
struct Outcomes {
    outcomes: Vec<u8>
}

impl Outcomes {
    pub fn from_answers_table(guess: &str, answers_table: &AnswersTable) -> Self {
        let bytes = guess.as_bytes();
        let outcomes = answers_table[0][(bytes[0] - b'a') as usize].iter()
            .zip(answers_table[1][(bytes[1] - b'a') as usize].iter())
            .zip(answers_table[2][(bytes[2] - b'a') as usize].iter())
            .zip(answers_table[3][(bytes[3] - b'a') as usize].iter())
            .zip(answers_table[4][(bytes[4] - b'a') as usize].iter())
            .map(|((((a, b), c), d), e)| {
                a * 81 + b * 27 + c * 9 + d * 3 + e
            })
            .collect();
        Outcomes { outcomes }
    }
    pub fn bucket(&self) -> Buckets {
        let mut buckets = [0; BUCKETS_SIZE];
        for outcome in &self.outcomes {
            buckets[*outcome as usize] += 1;
        }
        buckets
    }
}

pub struct WordleOpt<'a> {
    answers_table: AnswersTable,
    all_words: Vec<(AugmentedWord<'a>, Outcomes)>
}
type AnswersTable = [[Vec<u8>; 26]; 5];

fn build_answers_table(answers: &[AugmentedAnswer<'_>]) -> AnswersTable {
    let mut rv = [(); 5].map(|_| [(); 26].map(|_| Vec::with_capacity(answers.len())));
    for pos in 0..5 {
        for letter in 0..26 {
            let column = &mut rv[pos][letter];
            let ch = letter as u8 + b'a';
            let bitmask = 1 << letter;
            for answer in answers {
                let answer_ch = answer.word.as_bytes()[pos];
                let result = if answer_ch == ch {
                    2
                } else if answer.bitmask & bitmask != 0 {
                    1
                } else {
                    0
                };
                column.push(result);
            }
        }
    }
    rv
}

impl<'a> WordleOpt<'a> {
    pub fn new(words: &[&'a str], answers: &'a [&'a str]) -> Self {
        let ts = std::time::Instant::now();
        
        let aug_answers: Vec<_> = answers.iter().copied().map(AugmentedAnswer::new).collect();
        let answers_table = build_answers_table(&aug_answers);
        println!("{}", ts.elapsed().as_millis());
        let semi_answers: Vec<_> = words
        .iter()
        .chain(answers.iter())
        .copied()
        .map(|word| {
            (word, Outcomes::from_answers_table(word, &answers_table))
        }).collect();
        println!("{}", ts.elapsed().as_millis());
        let mut all_words: Vec<_> = semi_answers.into_iter().map(|(word, outcomes)| {
            (AugmentedWord { word, info: info(&outcomes.bucket()) }, outcomes)
        }).collect();
        // let mut all_words: Vec<_> = words
        //     .iter()
        //     .chain(answers.iter())
        //     .copied()
        //     .map(|word| {
        //         let outcomes = Outcomes::new(word, &aug_answers);
        //         (AugmentedWord { word, info: info(&outcomes.bucket()) }, outcomes)
        //     })
        //     .collect();
        all_words.sort_unstable_by_key(|(w, _)| OrderedFloat(-w.info));
        println!("{}", ts.elapsed().as_millis());
        WordleOpt { answers_table, all_words }
    }

    /// A list of all words, sorted by decreasing information
    pub fn all_words(&self) -> impl Iterator<Item=&AugmentedWord<'a>> {
        self.all_words.iter().map(|w| &w.0)
    }

    pub fn best_second_words(&self, first_word: &str) -> Vec<AugmentedWord> {
        let first_word_outcome = Outcomes::from_answers_table(first_word, &self.answers_table);
        let first_word_info = info(&first_word_outcome.bucket());
        let mut rv = Vec::with_capacity(self.all_words.len());
        let ts = std::time::Instant::now();
        let mut buckets = FxHashMap::default();
        let mut idxs = vec![];
        for (aug_word, second_outcome) in &self.all_words {
            if aug_word.word == first_word {
                continue;
            }
            
            let joint_info = info_two(&first_word_outcome, second_outcome, &mut buckets, &mut idxs);
            rv.push(AugmentedWord { word: aug_word.word, info: joint_info - first_word_info });
        }
        println!("{}", ts.elapsed().as_millis());
        rv.sort_unstable_by_key(|w| -OrderedFloat(w.info));
        rv
    }
    
    pub fn best_two(&self) -> Option<(&str, &str, f64)> {
        let best_info = AtomicF64::new(0.);
        let best_info_word = self.all_words.get(0).map(|w| w.0.info).unwrap_or_default();
        let i = AtomicU32::new(0);
        self.all_words.par_iter().filter_map(|(aug_word, outcomes)| {
            let cnt = i.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            let ts = std::time::Instant::now();
            if cnt % 100 == 0 {
                println!("{}, {}", cnt, aug_word.info);
            }
            let mut best_h = best_info.load(std::sync::atomic::Ordering::Acquire);
            let mut required_h = best_h - aug_word.info;
            if required_h > best_info_word {
                return None;
            }
            let mut best_word = None;
            let it = self.all_words.iter().skip_while(|(w, _)| w.info > aug_word.info || w.word != aug_word.word).skip(1);
            let mut buckets = FxHashMap::default();
            let mut idxs = vec![];
            for (other_word, other_outcome) in it {
                if other_word.info < required_h {
                    break;
                }
                let h = info_two(outcomes, other_outcome, &mut buckets, &mut idxs);
                if h > best_h {
                    best_h = best_info.load(std::sync::atomic::Ordering::Acquire);
                    if h > best_h {
                        best_h = h;
                        best_info.store(best_h, std::sync::atomic::Ordering::Release);
                        best_word = Some(other_word.word);
                        required_h = best_h - aug_word.info;
                    }
                }
            }
            if cnt % 100 == 0 {
                println!("{}", ts.elapsed().as_millis());
            }
            best_word.map(|best_word| {
                best_info.store(best_h, std::sync::atomic::Ordering::Release);
                (aug_word.word, best_word, best_h)
            })            
        }).inspect(|w| println!("{:?}", w))
        .max_by_key(|(_, _, h)| OrderedFloat(*h))
    }
}

