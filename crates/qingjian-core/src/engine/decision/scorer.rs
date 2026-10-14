//! 整批候选的分类决策分数；不与文本生成概率混用。
pub trait DecisionScorer: Send {
    /// 返回与 texts 同序的有限 logits；失败返回空数组。
    fn decide(&mut self, context: &str, pinyin: &str, texts: &[String]) -> Vec<f64>;
}
