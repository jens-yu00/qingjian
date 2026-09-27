//! 模型作用范围与过期请求的边界。
use super::*;
use crate::ranking::Scored;
use qingjian_dictionary::Match;

fn word(text: &'static str, pinyin: &'static str) -> Scored<'static> {
    Scored {
        hit: Match {
            text,
            pinyin,
            frequency: 10,
            exact: true,
        },
        full_last: true,
        coverage: 4,
        abbreviated: 0,
        weight: 0,
        penalty: 0.0,
    }
}
#[test]
fn ambiguous_segmentation_keeps_other_readings_in_their_slots() {
    let engine = engine().with_sentence_scorer(Box::new(Fresh), None, None, None);
    let mut words = vec![
        word("先", "xian"),
        word("西安", "xi an"),
        word("鲜", "xian"),
    ];
    engine.rescore_words(&mut words, "xian");
    assert_eq!(
        words.iter().map(|w| w.hit.text).collect::<Vec<_>>(),
        ["鲜", "西安", "先"]
    );
}
#[test]
fn new_query_drops_requests_for_previous_spelling() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers { gap: 20.0 }), None, None, None);
    assert_eq!(top(&engine), "是");
    assert!(engine.rescoring_pending());
    engine.set_input("shang");
    engine.query().unwrap();
    assert!(!engine.rescoring_pending());
}
#[test]
fn changing_context_invalidates_results_even_before_next_query() {
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(Prefers { gap: 20.0 }), None, None, None);
    top(&engine);
    engine.request_rescoring();
    // 等后台退出可确定结果已入队，不依赖 sleep 猜测线程进度。
    engine.rescorer.as_mut().unwrap().finish_for_test();
    engine.set_rescoring_context(Some("新上文".into()));
    assert!(!engine.poll_rescoring());
    assert_eq!(top(&engine), "是");
}

struct Fresh;
impl SentenceScorer for Fresh {
    fn score(&self, _: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|text| if *text == "鲜" { -1.0 } else { -20.0 })
            .collect()
    }
}
