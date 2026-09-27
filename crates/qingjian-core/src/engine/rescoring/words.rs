//! 仅在可靠上文下提升同读音词；不扩充候选或改变拼音结构。
use crate::engine::{Engine, Learner};
use crate::ranking::{Scored, weight_bonus};

const MAX_WORDS: usize = 12;
// 加权分领先至少 1 nat 才提升；小模型对近义同音词的微小差异不可靠。
const MIN_ADVANTAGE: f64 = 1.0;

impl Engine {
    pub(in crate::engine) fn rescore_words(&self, words: &mut [Scored<'_>], letters: &str) {
        if self.private
            || !self.has_sentence_scorer()
            || self.neural_weight <= 0.0
            || self.rescoring_context().trim().is_empty()
        {
            return;
        }
        let eligible = |word: &Scored<'_>| {
            word.hit.exact
                && word.full_last
                && word.abbreviated == 0
                && word.coverage == letters.len()
                && !word.altered()
        };
        let Some(first) = words.first().filter(|word| eligible(word)) else {
            return;
        };
        // 切分有歧义时只动首选所属的读音组，避免 xian 的「先」和「西安」互相挤占。
        let indices: Vec<usize> = words
            .iter()
            .enumerate()
            .filter(|(_, word)| eligible(word) && word.hit.pinyin == first.hit.pinyin)
            .map(|(index, _)| index)
            .take(MAX_WORDS)
            .collect();
        if indices.len() < 2 {
            return;
        }
        let texts: Vec<&str> = indices.iter().map(|&i| words[i].hit.text).collect();
        let Some(scores) = self.neural_scores(&texts) else {
            return;
        };
        let mut ranked: Vec<(usize, f64)> = indices
            .iter()
            .zip(scores)
            .map(|(&i, score)| {
                let word = &words[i];
                // 两张表通常记录同一次选择，取较大值避免迁移偏好重复加权。
                let habit = word
                    .weight
                    .max(self.learner.choice_weight(letters, word.hit.text));
                (i, self.neural_weight * score + weight_bonus(habit))
            })
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        let (winner, score) = ranked[0];
        if winner == indices[0] || score - ranked[1].1 < MIN_ADVANTAGE {
            return;
        }
        // 只提升确定胜出的一个词，其余同音词和其他切分保留相对顺序与位置。
        let promoted = words[winner];
        let position = indices
            .iter()
            .position(|&i| i == winner)
            .expect("selected word");
        for slot in (1..=position).rev() {
            words[indices[slot]] = words[indices[slot - 1]];
        }
        words[indices[0]] = promoted;
        self.last_rescored.set(true);
    }
}
