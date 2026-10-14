//! 用公开语境导出当前引擎的同音候选，供外部模型按同一集合做首选评测。
use std::path::Path;
use std::time::Instant;

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_lm::BigramModel;
use qingjian_neural::CharScorer;
use serde_json::Value;

fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let cases: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string("tools/context-bench/cases.json").expect("public cases"),
    )
    .expect("cases JSON");
    let scorer = CharScorer::load(Path::new("data/model/model.qjm")).expect("bundled model");
    let mut engine = Engine::new(Dictionary::from_path("data/generated/dict.qj").unwrap())
        .with_language_model(Box::new(
            BigramModel::from_path(Path::new("data/generated/lm.qj")).unwrap(),
        ))
        .with_sentence_scorer(Box::new(scorer), None, None, None);
    engine.set_strict_pinyin(true);
    let mut measured = Vec::new();
    for mut case in cases {
        let keys = case["pinyin"].as_str().unwrap();
        engine.set_input(keys);
        engine.set_rescoring_context(Some(case["before"].as_str().unwrap().to_owned()));
        let started = Instant::now();
        let query = engine.query().unwrap();
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        let first = &query.candidates.items[0].text;
        let candidates: Vec<_> = query
            .candidates
            .items
            .iter()
            .filter(|candidate| candidate.syllables.concat() == keys)
            .map(|candidate| candidate.text.clone())
            .take(12)
            .collect();
        assert!(
            candidates
                .iter()
                .any(|c| c == case["expected"].as_str().unwrap()),
            "expected word missing: {}",
            case["id"]
        );
        case["baseline_first"] = Value::from(first.clone());
        case["baseline_ms"] = Value::from(ms);
        case["candidates"] = serde_json::to_value(candidates).unwrap();
        println!(
            "{}: first={first}, expected={}, ms={ms:.2}",
            case["id"], case["expected"]
        );
        measured.push(case);
    }
    std::fs::write(output, serde_json::to_string_pretty(&measured).unwrap()).unwrap();
}
