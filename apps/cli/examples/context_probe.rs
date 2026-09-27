//! 用公开合成语境对照模型分数与最终候选；可选第二参数只读用户词频路径，不写回。
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
    let mut timings = Vec::new();
    for round in 0..10 {
        for (case, (context, keys, _)) in cases.iter().enumerate() {
            engine.set_input(keys);
            engine.set_rescoring_context(Some((*context).to_owned()));
            calls.store(0, Ordering::Relaxed);
            let start = Instant::now();
            let query = engine.query().expect("query");
            timings.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(
                query.candidates.items[0].text,
                ["上海", "伤害", "权利", "权力"][case]
            );
            let top: Vec<_> = query
                .candidates
                .items
                .iter()
                .take(5)
                .map(|c| c.text.as_str())
                .collect();
            if round == 0 {
                println!(
                    "engine context={context:?} keys={keys} model_calls={} top={top:?}",
                    calls.load(Ordering::Relaxed)
                );
            }
        }
    }
    timings.sort_by(f64::total_cmp);
    println!(
        "sync warmed query n={} p50_ms={:.2} p95_ms={:.2}",
        timings.len(),
        timings[timings.len() / 2],
        timings[(timings.len() * 95 / 100).saturating_sub(1)]
    );
    let scorer = CharScorer::load(Path::new(&model)).expect("async model");
    scorer.score("", &["的"]).expect("warm up");
    engine.set_async_sentence_scorer(Some(Box::new(CountedScorer { scorer, calls })));
    let mut initial_ms = Vec::new();
    let mut ready_ms = Vec::new();
    for _ in 0..5 {
        for (case, (context, keys, _)) in cases.iter().enumerate() {
            engine.set_input(keys);
            engine.set_rescoring_context(Some((*context).to_owned()));
            let start = Instant::now();
            let initial = engine.query().expect("initial query");
            initial_ms.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(
                initial.candidates.items[0].text,
                ["上海", "上海", "权力", "权力"][case]
            );
            assert!(engine.request_rescoring());
            while !engine.poll_rescoring() {
                assert!(start.elapsed().as_secs() < 5, "async result timeout");
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let updated = engine.query().expect("updated query");
            assert_eq!(
                updated.candidates.items[0].text,
                ["上海", "伤害", "权利", "权力"][case]
            );
            ready_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    initial_ms.sort_by(f64::total_cmp);
    ready_ms.sort_by(f64::total_cmp);
    println!(
        "async n={} initial_p95_ms={:.2} ready_p95_ms={:.2} (excludes shell debounce/render)",
        initial_ms.len(),
        initial_ms[18],
        ready_ms[18]
    );
    if let Some(user) = std::env::args().nth(2) {
        engine = engine.with_learner(Box::new(
            qingjian_learning::FrequencyLearner::from_path(user).expect("read learning data"),
        ));
        for (case, (context, keys, _)) in cases.iter().enumerate() {
            engine.set_input(keys);
            engine.set_rescoring_context(Some((*context).to_owned()));
            engine.query().expect("personal initial");
            assert!(engine.request_rescoring());
            let start = Instant::now();
            while !engine.poll_rescoring() {
                assert!(start.elapsed().as_secs() < 5);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let query = engine.query().expect("personal result");
            let matched = query.candidates.items[0].text == ["上海", "伤害", "权利", "权力"][case];
            println!("personal case={} expected_match={matched}", case + 1);
            assert!(matched, "personal context case {}", case + 1);
        }
    }
}
