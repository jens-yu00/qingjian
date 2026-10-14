//! 专用决策头的同音挑战者；排序权与失效边界由 Core 持有。
mod request;
mod scorer;
mod state;
#[cfg(test)]
mod tests;

use crate::engine::Engine;
use crate::ranking::Scored;
pub use scorer::DecisionScorer;
pub(crate) use state::DecisionState;

impl Engine {
    /// 专用决策器替换同音词的字级概率决策，后台串行执行。
    pub fn set_decision_scorer(&mut self, scorer: Option<Box<dyn DecisionScorer>>) {
        *self.decision.borrow_mut() = scorer.map(DecisionState::new);
    }

    pub fn has_decision_scorer(&self) -> bool {
        self.decision.borrow().is_some()
    }

    /// 壳转发选词导航事件；已展示的提升保留，未到达的结果不再获权。
    pub fn freeze_decision(&self) {
        if let Some(state) = self.decision.borrow_mut().as_mut() {
            state.freeze();
        }
    }

    pub(super) fn invalidate_decision(&self) {
        if let Some(state) = self.decision.borrow_mut().as_mut() {
            state.invalidate();
        }
    }

    pub(super) fn decision_words(&self, words: &mut [Scored<'_>], letters: &str) {
        if self.private || self.rescoring_context().trim().is_empty() {
            return;
        }
        let eligible = |w: &Scored<'_>| {
            w.hit.exact
                && w.full_last
                && w.abbreviated == 0
                && w.coverage == letters.len()
                && !w.altered()
        };
        let Some(first) = words.first().filter(|w| eligible(w)) else {
            return;
        };
        let mut seen = std::collections::HashSet::new();
        let indices: Vec<usize> = words
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                eligible(w) && w.hit.pinyin == first.hit.pinyin && seen.insert(w.hit.text)
            })
            .map(|(i, _)| i)
            .take(12)
            .collect();
        if indices.len() < 2 {
            return;
        }
        let mut state = self.decision.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        let texts = indices
            .iter()
            .map(|&i| words[i].hit.text.to_owned())
            .collect();
        let Some(scores) = state.prepare(self.rescoring_context(), letters, texts) else {
            return;
        };
        let mut order: Vec<usize> = (0..scores.len()).collect();
        order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
        let winner = order[0];
        let base = f64::from(words[indices[0]].hit.frequency) + 1.0;
        let challenger = f64::from(words[indices[winner]].hit.frequency) + 1.0;
        let threshold = if base > 100.0 * challenger { 2.0 } else { 1.0 };
        // 多个挑战者接近，不代表原首选仍可信；主要门槛针对原首选，极近平分另行保留原序。
        if winner == 0
            || scores[winner] - scores[0] < threshold
            || scores[winner] - scores[order[1]] < 0.1
        {
            return;
        }
        let promoted = words[indices[winner]];
        for slot in (1..=winner).rev() {
            words[indices[slot]] = words[indices[slot - 1]];
        }
        words[indices[0]] = promoted;
        state.promoted = Some(promoted.hit.text.into());
        self.last_rescored.set(true);
    }
}
