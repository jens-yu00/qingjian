//! 真实候选查询覆盖专用决策的排序、失效与隐私边界。
use super::{DecisionScorer, Engine};
use qingjian_dictionary::Dictionary;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Picks {
    gap: f64,
    calls: Arc<Mutex<Vec<Vec<String>>>>,
}
impl DecisionScorer for Picks {
    fn decide(&mut self, _: &str, _: &str, texts: &[String]) -> Vec<f64> {
        self.calls.lock().unwrap().push(texts.to_vec());
        texts
            .iter()
            .map(|s| if s == "食盐" { self.gap } else { 0.0 })
            .collect()
    }
}
fn engine(gap: f64) -> (Engine, Arc<Mutex<Vec<Vec<String>>>>) {
    let mut engine = Engine::new(Dictionary::parse(
        "实验\tshi yan\t9000\n食盐\tshi yan\t8000\n誓言\tshi yan\t7000\n试验品\tshi yan pin\t99999\n是\tshi\t9000\n"
    ).unwrap());
    engine.set_strict_pinyin(true);
    let calls = Arc::default();
    engine.set_decision_scorer(Some(Box::new(Picks {
        gap,
        calls: Arc::clone(&calls),
    })));
    engine.set_input("shiyan");
    engine.set_rescoring_context(Some("炒菜时加入少量".into()));
    (engine, calls)
}
fn top(e: &Engine) -> String {
    e.query().unwrap().candidates.items[0].text.clone()
}
fn wait(e: &mut Engine) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !e.poll_rescoring() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn immediate_baseline_then_single_legal_challenger() {
    let (mut e, calls) = engine(3.0);
    assert_eq!(top(&e), "实验");
    assert!(calls.lock().unwrap().is_empty());
    assert!(e.request_rescoring());
    wait(&mut e);
    assert_eq!(top(&e), "食盐");
    assert_eq!(top(&e), "食盐");
    assert!(!e.request_rescoring());
    assert_eq!(*calls.lock().unwrap(), vec![vec!["实验", "食盐", "誓言"]]);
}
#[test]
fn weak_or_invalid_decision_keeps_baseline() {
    for gap in [0.3, f64::NAN, f64::INFINITY] {
        let (mut e, _) = engine(gap);
        assert_eq!(top(&e), "实验");
        e.request_rescoring();
        wait(&mut e);
        assert_eq!(top(&e), "实验");
    }
}
#[test]
fn navigation_freezes_before_or_after_promotion() {
    let (mut e, _) = engine(3.0);
    top(&e);
    e.request_rescoring();
    e.freeze_decision();
    wait(&mut e);
    assert_eq!(top(&e), "实验");
    e.set_input("shiyan");
    top(&e);
    e.request_rescoring();
    wait(&mut e);
    assert_eq!(top(&e), "食盐");
    e.freeze_decision();
    assert_eq!(top(&e), "食盐");
}
#[test]
fn changed_input_context_or_session_never_reuses_old_batch() {
    for boundary in 0..4 {
        let (mut e, _) = engine(3.0);
        top(&e);
        e.request_rescoring();
        match boundary {
            0 => {
                e.push('a');
                e.backspace();
            }
            1 => {
                e.set_rescoring_context(Some("变化".into()));
                e.set_rescoring_context(Some("炒菜时加入少量".into()));
            }
            2 => {
                e.clear();
                e.set_input("shiyan");
                e.set_rescoring_context(Some("炒菜时加入少量".into()));
            }
            _ => e.swap_session(&mut Default::default()),
        }
        std::thread::sleep(Duration::from_millis(10));
        assert!(!e.poll_rescoring());
        if boundary < 3 {
            assert_eq!(top(&e), "实验");
        }
    }
}
#[test]
fn empty_context_private_and_abbreviated_input_do_not_request() {
    for boundary in 0..3 {
        let (mut e, calls) = engine(3.0);
        match boundary {
            0 => e.set_private(true),
            1 => e.set_rescoring_context(Some(String::new())),
            _ => e.set_input("shiy"),
        }
        top(&e);
        assert!(!e.request_rescoring());
        assert!(calls.lock().unwrap().is_empty());
    }
}

#[test]
fn overwhelming_dictionary_frequency_needs_stronger_context() {
    for (gap, expected) in [(1.5, "实验"), (2.5, "食盐")] {
        let calls = Arc::default();
        let mut e =
            Engine::new(Dictionary::parse("实验\tshi yan\t1000000\n食盐\tshi yan\t10\n").unwrap());
        e.set_decision_scorer(Some(Box::new(Picks { gap, calls })));
        e.set_input("shiyan");
        e.set_rescoring_context(Some("炒菜时加入少量".into()));
        top(&e);
        e.request_rescoring();
        wait(&mut e);
        assert_eq!(top(&e), expected);
    }
}

#[test]
fn promoted_acceptance_does_not_teach_a_global_preference() {
    use crate::Candidate;
    use crate::engine::Learner;
    use crate::sentence::Context;
    struct Learning(Arc<Mutex<usize>>);
    impl Learner for Learning {
        fn record(&mut self, _: &Candidate) {
            *self.0.lock().unwrap() += 1;
        }
        fn weight(&self, _: &str) -> u32 {
            0
        }
        fn record_choice(&mut self, _: &str, _: &str) {
            *self.0.lock().unwrap() += 1;
        }
        fn record_transition(&mut self, _: Context<'_>, _: &str, _: u32) {
            *self.0.lock().unwrap() += 1;
        }
        fn unrecord(&mut self, _: &str) {
            *self.0.lock().unwrap() += 1;
        }
        fn unrecord_choice(&mut self, _: &str, _: &str) {
            *self.0.lock().unwrap() += 1;
        }
    }
    let (e, _) = engine(3.0);
    let writes = Arc::new(Mutex::new(0));
    let mut e = e.with_learner(Box::new(Learning(Arc::clone(&writes))));
    top(&e);
    e.request_rescoring();
    wait(&mut e);
    let candidate = e.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "食盐");
    assert_eq!(e.commit(&candidate), "食盐");
    assert_eq!(*writes.lock().unwrap(), 0);
    e.note_backspace();
    e.note_backspace();
    e.set_input("shiyan");
    e.set_rescoring_context(Some("新上文".into()));
    let candidate = e.query().unwrap().candidates.items[0].clone();
    e.commit(&candidate);
    assert_eq!(
        *writes.lock().unwrap(),
        3,
        "正常选词仍可学习，未学习的模型词不能触发撤销扣旧次数"
    );
}

#[test]
fn nearby_challengers_do_not_rescue_a_clearly_rejected_baseline() {
    struct Scores(f64);
    impl DecisionScorer for Scores {
        fn decide(&mut self, _: &str, _: &str, _: &[String]) -> Vec<f64> {
            vec![0.0, 3.0, self.0]
        }
    }
    for (runner_up, expected) in [(2.8, "食盐"), (2.99, "实验")] {
        let (mut e, _) = engine(3.0);
        e.set_decision_scorer(Some(Box::new(Scores(runner_up))));
        top(&e);
        e.request_rescoring();
        wait(&mut e);
        assert_eq!(top(&e), expected);
    }
}
