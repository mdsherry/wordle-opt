use std::sync::atomic::Ordering;
use atomic_float::AtomicF64;
use rayon::prelude::*;

use ordered_float::OrderedFloat;

use crate::{AnswersTable, AugmentedAnswer, AugmentedWord, info, BucketType, MAX_FAST_LOG_ABSOLUTE_ERROR, fast_info};


const OUTCOME_COUNT: u16 = 9;
type Buckets = [u16; OUTCOME_COUNT as usize];

#[derive(Clone)]
pub struct Outcomes {
    outcomes: Vec<u8>,
    max: u8,
}

struct Outcomes2 {
    outcomes: Vec<u16>,
    max: u16,
}
impl Outcomes2 {
    pub fn new(outcomes1: &Outcomes, outcomes2: &Outcomes) -> Self {
        let mut xlate = [OUTCOME_COUNT * OUTCOME_COUNT; (OUTCOME_COUNT * OUTCOME_COUNT) as usize];
        let mut max = 0;
        let mut outcomes = vec![];
        for (&o1, &o2) in outcomes1.outcomes.iter().zip(&outcomes2.outcomes) {
            let idx = o1 as usize * outcomes1.max as usize + o2 as usize;
            if xlate[idx] == OUTCOME_COUNT * OUTCOME_COUNT {
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

#[cfg(test)]
mod test {
    use crate::approx::{AugmentedAnswer, build_answers_table, Outcomes};
    use crate::LetterIndex;

    #[test]
    fn test_augmented_answers() {
        let aug = AugmentedAnswer::new("brass");
        assert_eq!(aug.word, "brass");
        assert_eq!(aug.counts, [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_answers_table() {
        let aug1 = AugmentedAnswer::new("brass");
        let aug2 = AugmentedAnswer::new("arose");
        let table = build_answers_table(&[aug1, aug2]);
        assert_eq!(table[0]['a'.idx()][0], 1);
        assert_eq!(table[0]['b'.idx()][0], 1);
        assert_eq!(table[0]['s'.idx()][0], 2);
        assert_eq!(table[0]['c'.idx()][0], 0);
        assert_eq!(table[0]['a'.idx()][1], 1);
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
        let table = build_answers_table(&[aug3, aug4]);
        let raw_outcomes = Outcomes::uncompressed_outcomes("soare", &table);
        dbg!(&raw_outcomes);
        eprintln!("{}", raw_outcomes[0] as usize);
        eprintln!("{}", raw_outcomes[1] as usize);
        dbg!(AugmentedAnswer::new("allay").hits("soare"));
        dbg!(AugmentedAnswer::new("admit").hits("soare"));

        // assert_eq!(outcomes.outcomes[1], 122)
    }
}

impl Outcomes {
    pub fn new(mut outcomes: Vec<u8>) -> Self {
        let mut xlate = [OUTCOME_COUNT as u8; OUTCOME_COUNT as usize];
        let mut max = 0;
        for outcome in &mut outcomes {
            if xlate[*outcome as usize] == OUTCOME_COUNT as u8 {
                xlate[*outcome as usize] = max;
                *outcome = max as u8;
                max += 1;
            } else {
                *outcome = xlate[*outcome as usize];
            }
        }
        Outcomes { outcomes, max }
    }
    pub fn new_uncompressed(guess: &str, answers_table: &AnswersTable) -> Self {
        Outcomes { outcomes: Self::uncompressed_outcomes(guess, answers_table), max: 5 }
    }
    fn uncompressed_outcomes(guess: &str, answers_table: &AnswersTable) -> Vec<u8> {
        let bytes = guess.as_bytes();
        let mut counts = [0; 26];
        let mut outcomes = vec![0; answers_table[0][0].len()];
        for (b, answers_table) in bytes.iter().map(|b| *b - b'a').zip(answers_table) {
            // outcomes.iter_mut().for_each(|o| *o *= 3);
            counts[b as usize] += 1;
            for (outcome, blah) in outcomes.iter_mut().zip(&answers_table[b as usize]) {
                if *blah == -1 {
                    *outcome += 1;
                } else if *blah >= counts[b as usize] {
                    *outcome += 1;
                }
            }
        }
        // let outcomes = answers_table[0][(bytes[0] - b'a') as usize]
        //     .iter()
        //     .zip(answers_table[1][(bytes[1] - b'a') as usize].iter())
        //     .zip(answers_table[2][(bytes[2] - b'a') as usize].iter())
        //     .zip(answers_table[3][(bytes[3] - b'a') as usize].iter())
        //     .zip(answers_table[4][(bytes[4] - b'a') as usize].iter())
        //     .map(|((((a, b), c), d), e)| a + b + c + d + e)
        //     .collect();
        outcomes
    }

    fn from_answers_table(guess: &str, answers_table: &AnswersTable) -> Self {
        Self::new(Self::uncompressed_outcomes(guess, answers_table))
    }
    pub fn bucket(&self) -> Buckets {
        let mut buckets = [0; OUTCOME_COUNT as usize];
        for outcome in &self.outcomes {
            buckets[*outcome as usize] += 1;
        }
        buckets
    }
}



fn build_answers_table(answers: &[AugmentedAnswer<'_>]) -> AnswersTable {
    let mut rv = [(); 5].map(|_| [(); 26].map(|_| Vec::with_capacity(answers.len())));
    for (pos, word_pos) in rv.iter_mut().enumerate() {
        for (letter, column) in word_pos.iter_mut().enumerate().take(26) {
            let ch = letter as u8 + b'a';
            for answer in answers {
                let answer_ch = answer.word.as_bytes()[pos];
                let result = if answer_ch == ch {
                    1
                } else { answer.counts[letter] as i8 };
                column.push(result);
            }
        }
    }
    rv
}


#[derive(Clone)]
pub struct WordleOpt<'a> {
    answers_table: AnswersTable,
    aug_answers: Vec<AugmentedAnswer<'a>>,
    all_words: Vec<(AugmentedWord<'a>, Outcomes)>,
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
    pub fn pruned(&self, guess: &str, hits: usize) -> Self {
        let aug_answers: Vec<_> = self
            .aug_answers
            .iter()
            .filter(|a| a.word != guess && a.hits(guess) == hits)
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

    pub fn pruned_exact(&self, guess: &str, hint: u8) -> Self {
        let aug_answers: Vec<_> = self
            .aug_answers
            .iter()
            .filter(|a| a.word != guess && a.outcome(guess) == hint)
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

    pub fn answers_table(&self) -> &AnswersTable {
        &self.answers_table
    }

    /// A list of all words, sorted by decreasing information
    pub fn all_words(&self) -> impl Iterator<Item = &AugmentedWord<'a>> {
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
            rv.push(AugmentedWord {
                word: aug_word.word,
                info: joint_info - first_word_info,
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

    pub fn best_conditional_second(
        &self,
        first_word: &str,
    ) -> [Option<(AugmentedWord<'a>, usize)>; 6] {
        let mut rv = [None, None, None, None, None, None];
        let mut outcome_answers: [Vec<AugmentedAnswer>; 6] = [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];

        for answer in &self.aug_answers {
            if answer.word == first_word {
                continue;
            }
            let hits = answer.hits(first_word);
            outcome_answers[hits].push(answer.clone());
        }

        let answers_tables: [_; 6] = [
            build_answers_table(&outcome_answers[0]),
            build_answers_table(&outcome_answers[1]),
            build_answers_table(&outcome_answers[2]),
            build_answers_table(&outcome_answers[3]),
            build_answers_table(&outcome_answers[4]),
            build_answers_table(&outcome_answers[5]),
        ];
        for i in 0..6 {
            if outcome_answers[i].len() == 1 {
                rv[i] = Some((
                    AugmentedWord {
                        word: outcome_answers[i][0].word,
                        info: 0.,
                    },
                    1,
                ));
            } else {
                let semi_answers =
                    self.all_words()
                        .filter(|word| word.word != first_word)
                        .map(|word| {
                            (
                                word.word,
                                Outcomes::from_answers_table(word.word, &answers_tables[i]),
                            )
                        });
                let all_words: Vec<_> = semi_answers
                    .map(|(word, outcomes)| AugmentedWord {
                        word,
                        info: info(&outcomes.bucket()),
                    })
                    .collect();
                let word_count = outcome_answers[i].len();

                rv[i] = all_words
                    .into_iter()
                    .max_by_key(|w| OrderedFloat(w.info))
                    .map(|w| (w, word_count));
            }
        }

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