//! 严格全拼与个人英文尾段的交互；词表只含合成测试数据。
use crate::candidate::Candidate;
use crate::engine::{Engine, Learner};
use qingjian_dictionary::{Dictionary, WordList};

struct PersonalEnglish(WordList);
impl Learner for PersonalEnglish {
    fn record(&mut self, _: &Candidate) {}
    fn weight(&self, _: &str) -> u32 {
        0
    }
    fn user_english(&self) -> Option<&WordList> {
        Some(&self.0)
    }
}

fn mixed_engine() -> Engine {
    let dictionary = Dictionary::parse(
        "可能\tke neng\t5000\n技能\tji neng\t4000\n核能\the neng\t3000\n肯\tken\t9000\n金\tjin\t9000\n很\then\t9000\n额\te\t8000\n我\two\t9000\n我的\two de\t9000\n开发\tkai fa\t9000\n",
    ).unwrap();
    let mut engine = Engine::new(dictionary).with_learner(Box::new(PersonalEnglish(
        WordList::parse("ng\tng\t7\nrust\trust\t3740\nID\tid\t4610\n").unwrap(),
    )));
    engine.set_strict_pinyin(true);
    engine
}

#[test]
fn learned_english_suffix_cannot_hide_complete_pinyin_words() {
    let mut engine = mixed_engine();
    for (keys, word, syllables) in [
        ("keneng", "可能", ["ke", "neng"]),
        ("jineng", "技能", ["ji", "neng"]),
        ("heneng", "核能", ["he", "neng"]),
        ("ke'neng", "可能", ["ke", "neng"]),
    ] {
        engine.set_input(keys);
        let query = engine.query().unwrap();
        assert!(query.tail.is_empty(), "{keys}: {:?}", query.tail);
        assert_eq!(query.candidates.items[0].text, word, "{keys}");
        assert_eq!(query.candidates.items[0].syllables, syllables);
        assert_eq!(engine.commit(&query.candidates.items[0]), word);
        assert!(engine.composition().is_empty());
    }
}

#[test]
fn competing_sentence_considers_complete_alternative_segmentation() {
    let mut engine = mixed_engine();
    engine.set_input("wokeneng");
    assert!(engine.query().unwrap().tail.is_empty());
}

#[test]
fn exact_word_is_protected_even_if_mixed_score_is_higher() {
    let dictionary =
        Dictionary::parse("可能\tke neng\t1\n肯\tken\t100000\n额\te\t100000\n").unwrap();
    let mut engine =
        Engine::new(dictionary).with_english(WordList::parse("NG\tng\t9000\n").unwrap());
    engine.set_strict_pinyin(true);
    engine.set_input("keneng");
    let query = engine.query().unwrap();
    assert!(query.tail.is_empty());
    assert_eq!(query.candidates.items[0].text, "可能");
}

#[test]
fn genuine_english_tails_still_commit_the_entire_input() {
    let mut engine = mixed_engine();
    for (keys, text) in [
        ("kaifarust", "开发rust"),
        ("wodeid", "我的ID"),
        ("kaifang", "开发ng"),
    ] {
        engine.set_input(keys);
        let query = engine.query().unwrap();
        assert_eq!(query.candidates.items[0].text, text, "{keys}");
        assert_eq!(engine.commit(&query.candidates.items[0]), text);
        assert!(engine.composition().is_empty());
    }
}
