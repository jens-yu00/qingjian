//! 异步、无效模型结果及上文生命周期。
use super::*;
use std::time::{Duration, Instant};

#[test]
fn async_returns_baseline_then_promotes_with_complete_scores() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers { gap: 20.0 }), Some(0.5), None, None);
    assert_eq!(top(&engine), "是");
    assert!(engine.request_rescoring());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !engine.poll_rescoring() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(top(&engine), "事");
    engine.set_rescoring_context(Some("别处".into()));
    assert_eq!(top(&engine), "是");
}
#[test]
fn breaking_input_chain_clears_fallback_and_application_context() {
    let mut engine = sync(20.0);
    engine.history_mut().record("事情");
    engine.break_chain();
    assert!(engine.rescoring_context().is_empty());
    assert_eq!(top(&engine), "是");
}
struct Invalid(bool);
impl SentenceScorer for Invalid {
    fn score(&self, _: &str, texts: &[&str]) -> Vec<f64> {
        if self.0 {
            vec![f64::NAN; texts.len()]
        } else {
            vec![]
        }
    }
}
#[test]
fn invalid_scores_leave_original_order_and_cache_empty() {
    for nonfinite in [false, true] {
        let engine = engine().with_sentence_scorer(Box::new(Invalid(nonfinite)), None, None, None);
        assert_eq!(top(&engine), "是");
        assert!(engine.neural_cache.borrow().get("事").is_none());
    }
}
#[test]
fn private_input_does_not_queue_contextual_words() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers { gap: 20.0 }), None, None, None);
    engine.set_private(true);
    assert_eq!(top(&engine), "是");
    assert!(!engine.rescoring_pending());
}
