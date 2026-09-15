//! 全拼匹配策略与运行时切换。

use super::*;

impl Engine {
    /// 严格全拼：完整音节不补长、不改拼写；双拼、注音与英文模式不受影响。
    pub fn set_strict_pinyin(&mut self, enabled: bool) {
        if self.strict_pinyin != enabled {
            self.strict_pinyin = enabled;
            *self.correction_cache.borrow_mut() = None;
            self.forget_span_cache();
            self.cancel_prediction();
        }
    }

    pub(super) fn strict_pinyin_active(&self) -> bool {
        self.strict_pinyin && self.shuangpin.is_none() && !self.zhuyin && !self.english_mode
    }

    pub(super) fn pinyin_fuzzy(&self) -> FuzzyRules {
        if self.strict_pinyin_active() {
            FuzzyRules::default()
        } else {
            self.fuzzy
        }
    }

    /// 完整全拼的各个切法都保留（xian / xi an），但不再拆成简拼制造额外候选。
    pub(super) fn prefer_complete_pinyin(
        &self,
        mut parsed: Vec<Segmentation>,
    ) -> Vec<Segmentation> {
        if self.strict_pinyin_active() && parsed.iter().any(|s| s.incomplete_count() == 0) {
            parsed.retain(|s| s.incomplete_count() == 0);
        }
        parsed
    }

    pub(super) fn segment_pinyin(&self, text: &str) -> Result<Vec<Segmentation>, ParseError> {
        parser::segment(text).map(|parsed| self.prefer_complete_pinyin(parsed))
    }

    pub(super) fn segment_pinyin_prefix<'a>(
        &self,
        text: &'a str,
    ) -> Result<(Vec<Segmentation>, &'a str), ParseError> {
        segment_longest_prefix(text)
            .map(|(parsed, tail)| (self.prefer_complete_pinyin(parsed), tail))
    }

    /// 云端与本地共享音节边界，零编辑距离仍会允许 chu 补成 chuan，不能代替此检查。
    pub(super) fn strict_cloud_matches(&self, typed: &str, syllables: &[String]) -> bool {
        self.segment_pinyin(typed).is_ok_and(|parsed| {
            parsed.iter().any(|s| {
                s.syllables.len() == syllables.len()
                    && s.syllables.iter().zip(syllables).all(|(pattern, actual)| {
                        if pattern.complete {
                            pattern.text == *actual
                        } else {
                            actual.starts_with(&pattern.text)
                        }
                    })
            })
        })
    }
}
