// pub mod approx;

use std::sync::atomic::Ordering;

use atomic_float::AtomicF64;
use fast_math::log2_raw;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

pub const BUCKETS_SIZE: usize = 3 * 81;
type Buckets = [u16; BUCKETS_SIZE];
const MAX_FAST_LOG_ABSOLUTE_ERROR: f64 = 0.009;

pub fn info<'a>(buckets: impl IntoIterator<Item = &'a u16>) -> f64 {
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

fn fast_info<'a>(buckets: impl IntoIterator<Item = &'a u16>) -> f64 {
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
    idxs.extend(
        outcomes1
            .outcomes
            .iter()
            .zip(&outcomes2.outcomes)
            .map(|(&a, &b)| a as usize * outcomes2.max as usize + b as usize),
    );
    for idx in idxs.iter().copied() {
        buckets[idx] += 1;
    }
}

fn bucket_three(outcomes1_2: &Outcomes2, outcomes3: &Outcomes, buckets: &mut BucketType) {
    buckets.clear();
    buckets.resize(outcomes1_2.max as usize * outcomes3.max as usize, 0);
    let mut idxs = vec![];
    idxs.extend(
        outcomes1_2
            .outcomes
            .iter()
            .zip(&outcomes3.outcomes)
            .map(|(&a, &b)| a as usize * outcomes3.max as usize + b as usize),
    );
    for idx in idxs.iter().copied() {
        buckets[idx] += 1;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AugmentedWord<'a> {
    pub word: &'a str,
    pub info: f64,
}

#[derive(Debug, Clone)]
pub struct AugmentedAnswer<'a> {
    pub word: &'a str,
    counts: [u8; 26],
}

impl<'a> AugmentedAnswer<'a> {
    fn new(word: &'a str) -> Self {
        let mut counts = [0; 26];
        for ch in word.chars() {
            assert!(ch.is_ascii_lowercase());
            let b = ch as u8 - b'a';
            counts[b as usize] += 1;
        }

        Self { word, counts }
    }

    pub fn hits(&self, guess: &str) -> usize {
        let mut total = 0;
        let mut counts = [0; 26];
        for ch in guess.chars() {
            assert!(ch.is_ascii_lowercase());
            let b = ch as u8 - b'a';
            counts[b as usize] += 1;
            total += if counts[b as usize] <= self.counts[b as usize] {
                1
            } else {
                0
            };
        }
        total
    }

    pub fn outcome(&self, guess: &str) -> u8 {
        let mut rv = 0;
        let mut counts = [0; 26];
        for (g, a) in guess.chars().zip(self.word.chars()) {
            assert!(g.is_ascii_lowercase());
            let b = g as u8 - b'a';
            counts[b as usize] += 1;
            rv *= 3;
            rv += if g == a {
                2
            } else if counts[b as usize] <= self.counts[b as usize] {
                1
            } else {
                0
            };
        }
        rv
    }
    pub fn yellows(&self, guess: &str) -> u8 {
        let mut rv = 0;
        let mut counts = [0; 26];
        for (g, a) in guess.chars().zip(self.word.chars()) {
            assert!(g.is_ascii_lowercase());
            counts[g.idx()] += 1;
            rv += if g == a {
                0
            } else if counts[g.idx()] <= self.counts[g.idx()] {
                1
            } else {
                0
            };
        }
        rv
    }
    pub fn greens(&self, guess: &str) -> u8 {
        guess
            .chars()
            .zip(self.word.chars())
            .filter(|(a, b)| a == b)
            .count() as u8
    }
}

#[derive(Clone)]
pub struct Outcomes {
    pub outcomes: Vec<u8>,
    max: u8,
}

struct Outcomes2 {
    outcomes: Vec<u16>,
    max: u16,
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

pub fn bucket_label(mut bucket_idx: usize) -> String {
    assert!(
        bucket_idx < BUCKETS_SIZE,
        "Bucket index is too large: must be <= {BUCKETS_SIZE}, but was {bucket_idx}"
    );
    let mut rv = String::with_capacity(5);
    let e = bucket_idx % 3;
    bucket_idx /= 3;
    let d = bucket_idx % 3;
    bucket_idx /= 3;
    let c = bucket_idx % 3;
    bucket_idx /= 3;
    let b = bucket_idx % 3;
    bucket_idx /= 3;
    let a = bucket_idx % 3;
    let chars = ['_', '?', '!'];
    rv.push(chars[a]);
    rv.push(chars[b]);
    rv.push(chars[c]);
    rv.push(chars[d]);
    rv.push(chars[e]);
    rv
}

trait LetterIndex {
    fn idx(self) -> usize;
}
impl LetterIndex for char {
    fn idx(self) -> usize {
        ((self as u8) - b'a') as usize
    }
}

#[cfg(test)]
mod test {
    use crate::{bucket_label, build_answers_table, AugmentedAnswer, LetterIndex, Outcomes};

    #[test]
    fn test_augmented_answers() {
        let aug = AugmentedAnswer::new("brass");
        assert_eq!(aug.word, "brass");
        assert_eq!(
            aug.counts,
            [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_answers_table() {
        let aug1 = AugmentedAnswer::new("brass");
        let aug2 = AugmentedAnswer::new("arose");
        let table = build_answers_table(&[aug1, aug2]);
        assert_eq!(table[0]['a'.idx()][0], 1);
        assert_eq!(table[0]['b'.idx()][0], -1);
        assert_eq!(table[0]['s'.idx()][0], 2);
        assert_eq!(table[0]['c'.idx()][0], 0);
        assert_eq!(table[0]['a'.idx()][1], -1);
        assert_eq!(table[0]['b'.idx()][1], 0);
        assert_eq!(table[0]['s'.idx()][1], 1);
        assert_eq!(table[0]['c'.idx()][1], 0);
    }

    #[test]
    fn test_outcomes() {
        let aug1 = AugmentedAnswer::new("brass");
        let aug2 = AugmentedAnswer::new("arose");
        let aug3 = AugmentedAnswer::new("allay");
        let aug4 = AugmentedAnswer::new("admit");
        let answers_table = build_answers_table(&[aug1, aug2, aug3, aug4]);
        let raw_outcomes = Outcomes::uncompressed_outcomes("soare", &answers_table);
        dbg!(&raw_outcomes);
        eprintln!("{}", bucket_label(raw_outcomes[0] as usize));
        eprintln!("{}", bucket_label(raw_outcomes[1] as usize));
        dbg!(AugmentedAnswer::new("allay").outcome("soare"));
        dbg!(AugmentedAnswer::new("admit").outcome("soare"));

        // assert_eq!(outcomes.outcomes[1], 122)
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
    pub fn new_uncompressed(guess: &str, answers_table: &AnswersTable) -> Self {
        Outcomes {
            outcomes: Self::uncompressed_outcomes(guess, answers_table),
            max: 243,
        }
    }

    pub fn from_answers_table(guess: &str, answers_table: &AnswersTable) -> Self {
        Self::new(Self::uncompressed_outcomes(guess, answers_table))
    }

    fn uncompressed_outcomes(guess: &str, answers_table: &AnswersTable) -> Vec<u8> {
        let mut counts = [0; 26];
        let mut outcomes = vec![0; answers_table[0][0].len()];
        for (b, answers_table) in guess.chars().zip(answers_table) {
            outcomes.iter_mut().for_each(|o| *o *= 3);

            counts[b.idx()] += 1;
            for (outcome, answer_count) in outcomes.iter_mut().zip(&answers_table[b.idx()]) {
                if *answer_count == -1 {
                    *outcome += 2;
                } else if *answer_count >= counts[b.idx()] {
                    *outcome += 1;
                }
            }
        }

        outcomes
    }

    pub fn bucket(&self) -> Buckets {
        let mut buckets = [0; BUCKETS_SIZE];
        for outcome in &self.outcomes {
            buckets[*outcome as usize] += 1;
        }
        buckets
    }
}

#[derive(Clone)]
pub struct WordleOpt<'a> {
    answers_table: AnswersTable,
    aug_answers: Vec<AugmentedAnswer<'a>>,
    all_words: Vec<(AugmentedWord<'a>, Outcomes)>,
}
type AnswersTable = [[Vec<i8>; 26]; 5];

fn build_answers_table(answers: &[AugmentedAnswer<'_>]) -> AnswersTable {
    let mut rv = [(); 5].map(|_| [(); 26].map(|_| Vec::with_capacity(answers.len())));
    for (pos, word_pos) in rv.iter_mut().enumerate() {
        for (letter, column) in word_pos.iter_mut().enumerate().take(26) {
            let ch = letter as u8 + b'a';
            for answer in answers {
                let answer_ch = answer.word.as_bytes()[pos];
                let result = if answer_ch == ch {
                    -1
                } else {
                    answer.counts[letter] as i8
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
        let semi_answers = words
            .iter()
            .chain(answers.iter())
            .copied()
            .map(|word| (word, Outcomes::from_answers_table(word, &answers_table)));
        let mut all_words: Vec<_> = semi_answers
            .map(|(word, outcomes)| {
                (
                    AugmentedWord {
                        word,
                        info: info(&outcomes.bucket()),
                    },
                    outcomes,
                )
            })
            .collect();
        all_words.sort_unstable_by_key(|(w, _)| OrderedFloat(-w.info));
        WordleOpt {
            answers_table,
            all_words,
            aug_answers,
        }
    }
    pub fn answers(&self) -> &[AugmentedAnswer<'a>] {
        &self.aug_answers
    }

    fn pruned_generic(
        &self,
        guess: &str,
        answer_filter: impl Fn(&AugmentedAnswer) -> bool,
    ) -> Self {
        let aug_answers: Vec<_> = self
            .aug_answers
            .iter()
            .filter(|a| answer_filter(*a))
            .cloned()
            .collect();
        
        let answers_table = build_answers_table(&aug_answers);

        let semi_answers = self
            .all_words()
            .map(|aug| aug.word)
            .filter(|word| *word != guess)
            .map(|word| (word, Outcomes::from_answers_table(word, &answers_table)));
        let mut all_words: Vec<_> = semi_answers
            .map(|(word, outcomes)| {
                (
                    AugmentedWord {
                        word,
                        info: info(&outcomes.bucket()),
                    },
                    outcomes,
                )
            })
            .collect();
        all_words.sort_unstable_by_key(|(w, _)| OrderedFloat(-w.info));
        WordleOpt {
            answers_table,
            all_words,
            aug_answers,
        }
    }

    pub fn pruned(&self, guess: &str, hits: usize) -> Self {
        self.pruned_generic(guess, |a| a.word != guess && a.hits(guess) == hits)
    }
    pub fn pruned_exact(&self, guess: &str, hint: u8) -> Self {
        self.pruned_generic(guess, |a| a.word != guess && a.outcome(guess) == hint)
    }

    pub fn pruned_2(&self, guess: &str, yellow: u8, green: u8) -> Self {
        self.pruned_generic(guess, |a| {
            a.word != guess && a.greens(guess) == green && a.yellows(guess) == yellow
        })
    }

    pub fn answers_table(&self) -> &AnswersTable {
        &self.answers_table
    }

    /// A list of all words, sorted by decreasing information
    pub fn all_words(&self) -> impl Iterator<Item = &AugmentedWord<'a>> {
        self.all_words.iter().map(|w| &w.0)
    }

    pub fn best_second_words(&self, first_word: &str) -> Vec<AugmentedWord> {
        let first_word_outcome = Outcomes::from_answers_table(first_word, &self.answers_table());
        let first_word_info = info(&first_word_outcome.bucket());
        let mut rv = Vec::with_capacity(self.all_words.len());
        let mut buckets = BucketType::default();
        for (aug_word, second_outcome) in &self.all_words {
            if aug_word.word == first_word {
                continue;
            }

            bucket_two(&first_word_outcome, second_outcome, &mut buckets);
            let joint_info = info(&buckets);
            rv.push(AugmentedWord {
                word: aug_word.word,
                info: joint_info - first_word_info,
            });
        }
        rv.sort_unstable_by_key(|w| -OrderedFloat(w.info));
        rv
    }

    pub fn best_third_word(&self, first_word: &str, second_word: &str) -> Vec<AugmentedWord> {
        let first_word_outcome = Outcomes::from_answers_table(first_word, &self.answers_table());
        let second_word_outcome = Outcomes::from_answers_table(second_word, &self.answers_table());
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
            rv.push(AugmentedWord {
                word: aug_word.word,
                info: joint_info - joint_two_info,
            });
        }

        rv.sort_unstable_by_key(|w| -OrderedFloat(w.info));
        rv
    }

    pub fn pairs_above_threshold(&self, threshold: f64) -> Vec<(&str, &str, f64)> {
        let mut rv: Vec<_> = self
            .all_words
            .par_iter()
            .filter_map(|(aug_word, outcomes)| {
                if aug_word.info * 2. < threshold {
                    return None;
                }
                let required_h = threshold - aug_word.info;

                let it = self
                    .all_words
                    .iter()
                    .skip_while(|(w, _)| w.info > aug_word.info || w.word != aug_word.word)
                    .skip(1);
                let mut buckets = BucketType::default();
                let mut rv = vec![];
                for (other_word, other_outcome) in it {
                    if other_word.info < required_h {
                        break;
                    }
                    bucket_two(outcomes, other_outcome, &mut buckets);
                    let h = fast_info(&buckets);
                    if h + MAX_FAST_LOG_ABSOLUTE_ERROR > threshold {
                        let h = info(&buckets);

                        if h >= threshold {
                            rv.push((aug_word.word, other_word.word, h));
                        }
                    }
                }
                Some(rv)
            })
            .flatten()
            .collect();
        rv.sort_by_key(|(_, _, h)| OrderedFloat(-*h));
        rv
    }
    pub fn pairs_below_threshold(&self, threshold: f64) -> Vec<(&str, &str, f64)> {
        let all_words: Vec<_> = self.all_words.iter().rev().cloned().collect();
        let mut rv: Vec<_> = all_words
            .par_iter()
            .filter_map(|(aug_word, outcomes)| {
                if aug_word.info > threshold {
                    return None;
                }
                let it = all_words
                    .iter()
                    .skip_while(|(w, _)| w.info < aug_word.info || w.word != aug_word.word)
                    .skip(1);
                let mut buckets = BucketType::default();
                let mut rv = vec![];
                for (other_word, other_outcome) in it {
                    if other_word.info > threshold {
                        break;
                    }
                    bucket_two(outcomes, other_outcome, &mut buckets);
                    let h = fast_info(&buckets);
                    if h - MAX_FAST_LOG_ABSOLUTE_ERROR < threshold {
                        let h = info(&buckets);

                        if h <= threshold {
                            rv.push((aug_word.word, other_word.word, h));
                        }
                    }
                }
                Some(rv)
            })
            .flatten()
            .collect();
        rv.sort_by_key(|(_, _, h)| OrderedFloat(*h));
        rv
    }

    pub fn best_two(&self) -> Option<(&str, &str, f64)> {
        let best_info = AtomicF64::new(0.);
        // let best_info_word = self.all_words.get(0).map(|w| w.0.info).unwrap_or_default();
        self.all_words
            .par_iter()
            .filter_map(|(aug_word, outcomes)| {
                let mut best_h = best_info.load(Ordering::Acquire);
                let mut required_h = best_h - aug_word.info;
                if aug_word.info * 2. < best_h {
                    return None;
                }
                let mut best_word = None;
                let it = self
                    .all_words
                    .iter()
                    .skip_while(|(w, _)| w.info > aug_word.info || w.word != aug_word.word)
                    .skip(1);
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
