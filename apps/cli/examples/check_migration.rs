//! 离线验证迁移副本的学习权重、同音排序与严格全拼边界；仅输出汇总。

use std::collections::BTreeMap;
use std::path::Path;

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;

fn engine(dir: &Path) -> Engine {
    let dictionary = Dictionary::from_path("data/generated/dict.qj").expect("product dictionary");
    let learner = FrequencyLearner::from_path(dir.join("user.tsv")).expect("learning data");
    let mut engine = Engine::new(dictionary).with_learner(Box::new(learner));
    engine.set_strict_pinyin(true);
    engine
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(
        args.len(),
        2,
        "usage: check_migration BEFORE_DIR PREPARED_DIR"
    );
    let mut before = engine(Path::new(&args[0]));
    let mut after = engine(Path::new(&args[1]));
    let expected = std::fs::read_to_string(Path::new(&args[1]).join("expected-choices.tsv"))
        .expect("expected choices");
    let mut best: BTreeMap<&str, (&str, u32)> = BTreeMap::new();
    let mut verified = 0;
    for line in expected.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "expected columns");
        let count: u32 = fields[2].parse().expect("choice count");
        assert!(after.learner().choice_weight(fields[0], fields[1]) >= count);
        verified += 1;
        let entry = best.entry(fields[0]).or_insert((fields[1], count));
        if count > entry.1 {
            *entry = (fields[1], count);
        }
    }
    let mut sample: Vec<_> = best.into_iter().collect();
    sample.sort_by_key(|(_, (_, count))| std::cmp::Reverse(*count));
    sample.truncate(128);
    let mut improved = 0;
    let mut before_first = 0;
    let mut after_first = 0;
    for (keys, (word, _)) in &sample {
        before.set_input(keys);
        after.set_input(keys);
        let old = before.query().expect("before query");
        let new = after.query().expect("after query");
        let rank = |items: &[qingjian_core::Candidate]| {
            items
                .iter()
                .position(|c| c.text == *word)
                .unwrap_or(usize::MAX)
        };
        let old_rank = rank(&old.candidates.items);
        let new_rank = rank(&new.candidates.items);
        assert!(new_rank < usize::MAX, "imported preferred candidate absent");
        improved += usize::from(new_rank < old_rank);
        before_first += usize::from(old_rank == 0);
        after_first += usize::from(new_rank == 0);
    }
    assert!(improved > 0, "no sampled preference improved");
    after.set_input("liuchu");
    let query = after.query().expect("strict query");
    assert!(
        query
            .candidates
            .items
            .iter()
            .all(|c| !matches!(c.text.as_str(), "流传" | "留长" | "流畅"))
    );
    println!(
        "verified_choices={verified}; sampled_inputs={}; improved_ranks={improved}; preferred_first_before={before_first}; preferred_first_after={after_first}; strict_pinyin=passed",
        sample.len()
    );
}
