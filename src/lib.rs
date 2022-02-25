use std::sync::atomic::Ordering;

use atomic_float::AtomicF64;
use fast_math::log2_raw;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

const BUCKETS_SIZE: usize = 3 * 81;
type Buckets = [u16; BUCKETS_SIZE];
const MAX_FAST_LOG_ABSOLUTE_ERROR: f64 = 0.009;

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

fn fast_info<'a>(buckets: impl IntoIterator<Item=&'a u16>) -> f64 {
    let mut total = 0.;
    let mut h = 0.;
    for &bucket in buckets {
        if bucket > 0 {
            // let p = (bucket as f64) / total;
            let p = bucket as f64;
            total += p;
            h += p * log2_raw(p as f32) as f64;
        }
    }
    h = h / total - total.log2();
    -h
}
type BucketType = Vec<u16>;
fn bucket_two(outcomes1: &Outcomes, outcomes2: &Outcomes, buckets: &mut BucketType) {
    buckets.clear();
    buckets.resize(outcomes1.max as usize * outcomes2.max as usize, 0);
    let mut idxs = vec![];
    idxs.extend(outcomes1.outcomes.iter().zip(&outcomes2.outcomes).map(|(&a, &b)| a as usize * outcomes2.max as usize + b as usize));
    for idx in idxs.iter().copied() {
        buckets[idx] += 1;
    }
}

fn bucket_three(outcomes1_2: &Outcomes2, outcomes3: &Outcomes, buckets: &mut BucketType) {
    buckets.clear();
    buckets.resize(outcomes1_2.max as usize * outcomes3.max as usize, 0);
    let mut idxs = vec![];
    idxs.extend(outcomes1_2.outcomes.iter().zip(&outcomes3.outcomes).map(|(&a, &b)| a as usize * outcomes3.max as usize + b as usize));
    for idx in idxs.iter().copied() {
        buckets[idx] += 1;
    }
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
    outcomes: Vec<u8>,
    max: u8
}

struct Outcomes2 {
    outcomes: Vec<u16>,
    max: u16
}
impl Outcomes2 {
    pub fn new(outcomes1: &Outcomes, outcomes2: &Outcomes) -> Self {
        let mut xlate = [243 * 243; 243 * 243];
        let mut max = 0;
        let mut outcomes = vec![];
        for (&o1, &o2) in outcomes1.outcomes.iter().zip(&outcomes2.outcomes) {
            let idx = o1 as usize * outcomes1.max as usize + o2 as usize;
            if xlate[idx] == 243 * 243 {
                xlate[idx] = max;
                outcomes.push(max);
                max += 1;
            } else {
                outcomes.push(xlate[idx]);
            }
        }
        Outcomes2 { outcomes, max }
    }
}

impl Outcomes {
    pub fn new(mut outcomes: Vec<u8>) -> Self {
        let mut xlate = [243; 243];
        let mut max = 0;
        for outcome in &mut outcomes {
            if xlate[*outcome as usize] == 243 {
                xlate[*outcome as usize] = max;
                *outcome = max;
                max += 1;
            } else {
                *outcome = xlate[*outcome as usize];
            }
        }
        Outcomes { outcomes, max }
    }
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
        Self::new(outcomes)
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
    for (pos, word_pos) in rv.iter_mut().enumerate() {
        for (letter, column) in word_pos.iter_mut().enumerate().take(26) {
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
        let aug_answers: Vec<_> = answers.iter().copied().map(AugmentedAnswer::new).collect();
        let answers_table = build_answers_table(&aug_answers);
        let semi_answers= words
            .iter()
            .chain(answers.iter())
            .copied()
            .map(|word| {
                (word, Outcomes::from_answers_table(word, &answers_table))
            });
        let mut all_words: Vec<_> = semi_answers.map(|(word, outcomes)| {
            (AugmentedWord { word, info: info(&outcomes.bucket()) }, outcomes)
        }).collect();
        all_words.sort_unstable_by_key(|(w, _)| OrderedFloat(-w.info));
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
        let mut buckets = BucketType::default();
        for (aug_word, second_outcome) in &self.all_words {
            if aug_word.word == first_word {
                continue;
            }
            
            bucket_two(&first_word_outcome, second_outcome, &mut buckets);
            let joint_info = info(&buckets);
            rv.push(AugmentedWord { word: aug_word.word, info: joint_info - first_word_info });
        }
        rv.sort_unstable_by_key(|w| -OrderedFloat(w.info));
        rv
    }
    
    pub fn best_third_word(&self, first_word: &str, second_word: &str) -> Vec<AugmentedWord> {
        let first_word_outcome = Outcomes::from_answers_table(first_word, &self.answers_table);
        let second_word_outcome = Outcomes::from_answers_table(second_word, &self.answers_table);
        let joint_outcomes = Outcomes2::new(&first_word_outcome, &second_word_outcome);
        let mut rv = Vec::with_capacity(self.all_words.len());
        let mut buckets = BucketType::default();
        bucket_two(&first_word_outcome, &second_word_outcome, &mut buckets);
        let joint_two_info = info(&buckets);
        for (aug_word, third_outcome) in &self.all_words {
            if aug_word.word == first_word {
                continue;
            }
            
            bucket_three(&joint_outcomes, third_outcome, &mut buckets);
            let joint_info = info(&buckets);
            rv.push(AugmentedWord { word: aug_word.word, info: joint_info - joint_two_info });
        }
        
        rv.sort_unstable_by_key(|w| -OrderedFloat(w.info));
        rv
    }

    pub fn best_two(&self) -> Option<(&str, &str, f64)> {
        let best_info = AtomicF64::new(0.);
        let best_info_word = self.all_words.get(0).map(|w| w.0.info).unwrap_or_default();
        self.all_words.par_iter().filter_map(|(aug_word, outcomes)| {
             let mut best_h = best_info.load(Ordering::Acquire);
            let mut required_h = best_h - aug_word.info;
            if required_h > best_info_word {
                return None;
            }
            let mut best_word = None;
            let it = self.all_words.iter().skip_while(|(w, _)| w.info > aug_word.info || w.word != aug_word.word).skip(1);
            let mut buckets = BucketType::default();
            for (other_word, other_outcome) in it {
                if other_word.info < required_h {
                    break;
                }
                bucket_two(outcomes, other_outcome, &mut buckets);
                let h = fast_info(&buckets);
                if h + MAX_FAST_LOG_ABSOLUTE_ERROR > best_h {
                    let h = info(&buckets);
                    if h > best_h {
                        best_h = h;
                        best_word = Some(other_word.word);
                        required_h = best_h - aug_word.info;
                    }
                }
            }
            best_word.and_then(|best_word| {
                let current_best = best_info.load(Ordering::Acquire);
                if current_best < best_h {
                    best_info.store(best_h, Ordering::Release);
                    Some((aug_word.word, best_word, best_h))
                } else {
                    None
                }
            })            
        })
        .max_by_key(|(_, _, h)| OrderedFloat(*h))
    }
}

