//! 通过真实查询验证同音词重排与回退边界。
use crate::sentence::SentenceScorer;
mod bounds;
mod lifecycle;

use super::*;
use crate::engine::Learner;
use crate::{Candidate, CustomPhrase};
use qingjian_dictionary::Dictionary;

struct Prefers {
    gap: f64,
}
impl SentenceScorer for Prefers {
    fn score(&self, context: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|text| {
                if context == "事情" && *text == "事" {
                    -1.0
                } else {
                    -1.0 - self.gap
                }
            })
            .collect()
    }
}
struct Habit;
impl Learner for Habit {
    fn record(&mut self, _: &Candidate) {}
    fn weight(&self, text: &str) -> u32 {
        if text == "是" { 10000 } else { 0 }
    }
    fn choice_weight(&self, _: &str, text: &str) -> u32 {
        self.weight(text)
    }
}
fn engine() -> Engine {
    let mut engine = Engine::new(
        Dictionary::parse(
            "是\tshi\t9000\n事\tshi\t8000\n市\tshi\t7000\n是啊\tshi a\t9900\n上\tshang\t1000\n",
        )
        .unwrap(),
    )
    .with_learner(Box::new(Habit));
    engine.set_strict_pinyin(true);
    engine.set_input("shi");
    engine.set_rescoring_context(Some("事情".into()));
    engine
}
fn sync(gap: f64) -> Engine {
    engine().with_sentence_scorer(Box::new(Prefers { gap }), Some(0.5), None, None)
}
fn top(engine: &Engine) -> String {
    engine.query().unwrap().candidates.items[0].text.clone()
}

#[test]
fn strong_context_beats_capped_personal_habit() {
    assert_eq!(top(&engine()), "是");
    assert_eq!(top(&sync(6.0)), "事");
}
#[test]
fn weak_context_and_unavailable_context_keep_habit() {
    assert_eq!(top(&sync(1.0)), "是");
    let mut engine = sync(20.0);
    engine.set_rescoring_context(Some(String::new()));
    assert_eq!(top(&engine), "是");
    engine.set_rescoring_context(None);
    engine.history_mut().record("事情");
    assert_eq!(top(&engine), "事");
}
#[test]
fn custom_phrase_keeps_fixed_slot() {
    let mut engine = sync(20.0);
    engine
        .set_custom_phrases(vec![CustomPhrase {
            code: "shi".into(),
            text: "固定内容".into(),
            position: 1,
            enabled: true,
        }])
        .unwrap();
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "固定内容");
    assert_eq!(query.candidates.items[1].text, "事");
}
#[test]
fn incomplete_spelling_and_strict_membership_stay_unchanged() {
    let mut baseline = engine();
    let mut neural = sync(20.0);
    for keys in ["sh", "s", "shia"] {
        baseline.set_input(keys);
        neural.set_input(keys);
        let a = baseline.query().unwrap();
        let b = neural.query().unwrap();
        assert_eq!(a.candidates.items, b.candidates.items, "{keys}");
    }
    neural.set_input("shi");
    let query = neural.query().unwrap();
    assert!(!query.candidates.items.iter().any(|c| c.text == "是啊"));
}
