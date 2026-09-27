//! 用公开合成语境对照模型分数与最终候选；不读取或写入个人学习数据。
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use qingjian_core::{Engine, sentence::SentenceScorer};
use qingjian_dictionary::Dictionary;
use qingjian_lm::BigramModel;
use qingjian_neural::CharScorer;

struct CountedScorer {
    /// 真实随包模型。
    scorer: CharScorer,
    /// 验证候选阶段是否实际调用了模型。
    calls: Arc<AtomicUsize>,
}
impl SentenceScorer for CountedScorer {
    fn score(&self, context: &str, texts: &[&str]) -> Vec<f64> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.scorer.score(context, texts).expect("model score")
    }
}
fn main() {
    let model = std::env::args().nth(1).expect("model.qjm path");
    let scorer = CharScorer::load(Path::new(&model)).expect("model");
    println!("model_metadata={:?}", scorer.metadata());
    let cases = [
        ("我明天坐高铁去", "shanghai", ["上海", "伤害"]),
        ("这种行为会对孩子造成", "shanghai", ["上海", "伤害"]),
        ("每个公民都享有平等的", "quanli", ["权利", "权力"]),
        ("他掌握着至高无上的", "quanli", ["权利", "权力"]),
    ];
    for (context, keys, words) in cases {
        let start = Instant::now();
        let scores = scorer.score(context, &words).expect("score");
        println!(
            "model context={context:?} keys={keys} words={words:?} scores={scores:?} ms={:.1}",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engine = Engine::new(Dictionary::from_path("data/generated/dict.qj").unwrap())
        .with_language_model(Box::new(
            BigramModel::from_path(Path::new("data/generated/lm.qj")).unwrap(),
        ))
        .with_sentence_scorer(
            Box::new(CountedScorer {
                scorer,
                calls: calls.clone(),
            }),
            None,
            None,
            None,
        );
    engine.set_strict_pinyin(true);
    for (context, keys, _) in cases {
        engine.set_input(keys);
        engine.set_rescoring_context(Some(context.to_owned()));
        calls.store(0, Ordering::Relaxed);
        let query = engine.query().expect("query");
        let top: Vec<_> = query
            .candidates
            .items
            .iter()
            .take(5)
            .map(|c| c.text.as_str())
            .collect();
        println!(
            "engine context={context:?} keys={keys} model_calls={} top={top:?}",
            calls.load(Ordering::Relaxed)
        );
    }
}
