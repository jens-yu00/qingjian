//! 神经重打分：整句转换的前几条路径交给字级模型（[`crate::sentence::SentenceScorer`]）再排一次。
//!
//! 打分有两种接法：同步的（[`Engine::with_sentence_scorer`]，查询里当场打，CLI 评测用）和异步的
//! （[`Engine::with_async_sentence_scorer`]，后台线程；壳里用）。两种都经过一张「前文 + 文本 → 神经分」的缓存
//! （[`NeuralCache`]）：同步时缺的分当场补进去，异步时缺的先记下来，壳在用户停顿后调 [`Engine::request_rescoring`]
//! 一次送去后台，[`Engine::poll_rescoring`] 收到结果后再查一次，这时全部路径的分都在缓存里，排序自然换成重排后的。
//! 按键回调永远不等模型：先按词级模型出候选，模型的意见晚几十毫秒到。

mod cache;
mod words;
mod worker;

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod tests;

use super::{Engine, take_last_chars};
use crate::sentence::Conversion;

pub(crate) use cache::NeuralCache;
pub(crate) use worker::RescoreWorker;

impl Engine {
    /// 接了重打分器（同步或异步）。
    pub fn has_sentence_scorer(&self) -> bool {
        self.sentence_scorer.is_some()
            || self.rescorer.as_ref().is_some_and(RescoreWorker::is_alive)
    }

    /// 给模型看的前文：壳给了应用里的光标前文就用它（[`Self::set_rescoring_context`]），
    /// 否则用本会话最近上屏的字符；长度按 `neural_context` 截。
    pub(super) fn rescoring_context(&self) -> String {
        if self.neural_context == 0 {
            return String::new();
        }
        match &self.rescoring_before {
            Some(before) => take_last_chars(before, self.neural_context),
            None => self.history.recent(self.neural_context).to_owned(),
        }
    }

    /// 壳告知应用里光标前的文本（每次查询前给；应用给不出就 `None`，退回本会话历史）。
    pub fn set_rescoring_context(&mut self, before: Option<String>) {
        self.rescoring_before = before;
        let context = self.rescoring_context();
        tracing::debug!(
            application_context = self.rescoring_before.is_some(),
            chars = context.chars().count(),
            "本地模型上文"
        );
        self.neural_cache.borrow_mut().ensure_context(&context);
    }

    /// 把几条整句路径按「路径分 + λ·(神经分 − 静态分)」重排。缓存里缺分的：同步打分器当场补，异步的先记下等壳来取；
    /// 有任何一条没分就不动顺序（半截重排比不重排还糟）。
    pub(super) fn rescore_paths(&self, paths: &mut [Conversion]) {
        if paths.len() < 2 || !self.has_sentence_scorer() {
            return;
        }
        let texts: Vec<&str> = paths.iter().map(|p| p.text.as_str()).collect();
        let Some(scores) = self.neural_scores(&texts) else {
            return;
        };
        let lambda = self.neural_weight;
        for (path, neural) in paths.iter_mut().zip(scores) {
            path.score += lambda * (neural - path.static_score);
        }
        paths.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.last_rescored.set(true);
    }

    /// 整批分数齐全才可比较；异步查询只登记缺项，不等待模型。
    fn neural_scores(&self, texts: &[&str]) -> Option<Vec<f64>> {
        let context = self.rescoring_context();
        let mut cache = self.neural_cache.borrow_mut();
        cache.ensure_context(&context);
        let mut missing: Vec<String> = Vec::new();
        for text in texts {
            if cache.get(text).is_none() && !missing.iter().any(|m| m == text) {
                missing.push((*text).to_owned());
            }
        }
        if !missing.is_empty() {
            match &self.sentence_scorer {
                Some(scorer) => {
                    let texts: Vec<&str> = missing.iter().map(String::as_str).collect();
                    let scores = scorer.score(&context, &texts);
                    if scores.len() != texts.len() || scores.iter().any(|s| !s.is_finite()) {
                        return None;
                    }
                    for (text, score) in texts.iter().zip(scores) {
                        cache.insert(text, score);
                    }
                }
                None => {
                    for text in &missing {
                        cache.want(text);
                    }
                    return None;
                }
            }
        }
        texts.iter().map(|text| cache.get(text)).collect()
    }

    /// 最近一次查询里有候选还没拿到神经分：壳该在用户停顿后调 [`Self::request_rescoring`]。
    pub fn rescoring_pending(&self) -> bool {
        self.rescorer.is_some() && self.neural_cache.borrow().has_wanted()
    }

    /// 把攒着的文本送去后台打分。没接异步打分器或没什么要打的返回 `false`。
    pub fn request_rescoring(&mut self) -> bool {
        let Some(worker) = &self.rescorer else {
            return false;
        };
        let mut cache = self.neural_cache.borrow_mut();
        let wanted = cache.take_wanted();
        if wanted.is_empty() {
            return false;
        }
        tracing::debug!(texts = wanted.len(), "神经重打分请求");
        worker.submit(cache.context().to_owned(), wanted);
        true
    }

    /// 收后台打好的分。有新分进了缓存返回 `true`，壳该重新 [`Self::query`] 一次；前文已经变了的结果丢掉。
    pub fn poll_rescoring(&mut self) -> bool {
        let Some(worker) = &self.rescorer else {
            return false;
        };
        let mut updated = false;
        while let Some(scored) = worker.poll() {
            let mut cache = self.neural_cache.borrow_mut();
            if scored.context != cache.context()
                || scored.scores.len() != scored.texts.len()
                || scored.scores.iter().any(|s| !s.is_finite())
            {
                continue;
            }
            for (text, score) in scored.texts.iter().zip(scored.scores) {
                cache.insert(text, score);
            }
            updated = true;
        }
        updated
    }
}
