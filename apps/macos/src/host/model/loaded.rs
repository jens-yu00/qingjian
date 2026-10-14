//! 已预热的本地模型；壳只选择后端，不参与候选决策。
use qingjian_neural::{CharScorer, decision::SolScorer};

pub(in crate::host) enum LoadedModel {
    Sentence(Box<CharScorer>),

    Decision(SolScorer),
}
