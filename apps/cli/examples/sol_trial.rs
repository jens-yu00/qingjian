//! 通过产品 Core 与真实常驻 Sol 工作进程回放公开语境。
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_lm::BigramModel;
use qingjian_neural::decision::SolScorer;
use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let started = Instant::now();
    let scorer = SolScorer::load(Path::new(&args[1])).expect("Sol ready");
    let load_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut engine = Engine::new(Dictionary::from_path("data/generated/dict.qj").unwrap())
        .with_language_model(Box::new(
            BigramModel::from_path(Path::new("data/generated/lm.qj")).unwrap(),
        ));
    engine.set_strict_pinyin(true);
    engine.set_decision_scorer(Some(Box::new(scorer)));
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let mut rows = Vec::new();
    for mut case in cases {
        engine.set_input(case["pinyin"].as_str().unwrap());
        engine.set_rescoring_context(Some(case["before"].as_str().unwrap().into()));
        let start = Instant::now();
        let first = engine.query().unwrap().candidates.items[0].text.clone();
        let baseline_ms = start.elapsed().as_secs_f64() * 1000.0;
        let requested = engine.request_rescoring();
        let start = Instant::now();
        let mut completed = false;
        if requested {
            while start.elapsed() < Duration::from_secs(3) {
                if engine.poll_rescoring() {
                    completed = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        let decision_ms = start.elapsed().as_secs_f64() * 1000.0;
        let query = engine.query().unwrap();
        let selected = query.candidates.items[0].text.clone();
        case["baseline_first"] = json!(first);
        case["selected"] = json!(selected);
        case["baseline_ms"] = json!(baseline_ms);
        case["decision_ms"] = json!(decision_ms);
        case["requested"] = json!(requested);
        case["completed"] = json!(completed);
        case["candidates"] = json!(
            query
                .candidates
                .items
                .iter()
                .take(12)
                .map(|c| &c.text)
                .collect::<Vec<_>>()
        );
        println!("{}: {first} -> {selected}, {decision_ms:.0} ms", case["id"]);
        rows.push(case);
    }
    std::fs::write(
        &args[3],
        serde_json::to_string_pretty(&json!({"load_ms":load_ms,"rows":rows})).unwrap(),
    )
    .unwrap();
}
