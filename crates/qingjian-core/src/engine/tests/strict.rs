//! 严格全拼的候选范围、上屏与切换契约。

use super::*;

fn dictionary() -> Dictionary {
    Dictionary::parse("流出\tliu chu\t1000\n留出\tliu chu\t500\n六畜\tliu chu\t100\n流传\tliu chuan\t90000\n流畅\tliu chang\t80000\n流处理\tliu chu li\t10000\n流\tliu\t10000\n出\tchu\t10000\n").unwrap()
}

#[test]
fn complete_pinyin_excludes_completion_and_abbreviation_reinterpretation() {
    let mut engine = Engine::new(dictionary());
    engine.set_strict_pinyin(true);
    engine.set_input("liuchu");
    let query = engine.query().unwrap();
    let texts: Vec<_> = query
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    for expected in ["流出", "留出", "六畜", "流"] {
        assert!(texts.contains(&expected), "missing {expected}: {texts:?}");
    }
    for unexpected in ["流传", "流畅", "流处理"] {
        assert!(
            !texts.contains(&unexpected),
            "unexpected {unexpected}: {texts:?}"
        );
    }
    assert!(
        query
            .segmentations
            .iter()
            .all(|s| s.incomplete_count() == 0)
    );
}

#[test]
fn incomplete_pinyin_completes_and_commit_learns_the_typed_code() {
    let mut engine =
        Engine::new(dictionary()).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_strict_pinyin(true);
    for input in ["liuch", "lc"] {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == "流传")
            .expect("incomplete input still completes");
        assert_eq!(engine.commit(&candidate), "流传");
        assert!(engine.composition().is_empty());
        assert_eq!(engine.learner().choice_weight(input, "流传"), 1);
    }
    engine.set_input("liuchuan");
    assert!(texts_of(&engine).contains(&"流传".to_owned()));
}

#[test]
fn complete_segmentation_ambiguity_and_prefix_commit_are_preserved() {
    let mut engine = engine();
    engine.set_strict_pinyin(true);
    engine.set_input("xian");
    let texts = texts_of(&engine);
    assert!(texts.contains(&"先".to_owned()) && texts.contains(&"西安".to_owned()));
    engine.set_input("kaifazhe");
    let kai = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "开")
        .unwrap();
    engine.commit(&kai);
    assert_eq!(engine.composition().text(), "fazhe");
}

#[test]
fn strict_mode_disables_legal_syllable_typo_edges() {
    let mut engine = Engine::new(Dictionary::parse("没关系\tmei guan xi\t800000\n关系\tguan xi\t500000\n没\tmei\t900000\n干\tgan\t200000\n洗\txi\t100000\n美感\tmei gan\t300000\n").unwrap());
    engine.set_input("meiganxi");
    assert_eq!(texts_of(&engine)[0], "没关系");
    engine.set_strict_pinyin(true);
    assert!(!texts_of(&engine).contains(&"没关系".to_owned()));
    engine.set_strict_pinyin(false);
    assert_eq!(texts_of(&engine)[0], "没关系");
}

#[test]
fn switching_strict_mode_invalidates_whole_string_correction() {
    let mut engine = Engine::new(Dictionary::parse("你好吗\tni hao ma\t5000\n你好\tni hao\t9000\n你\tni\t90000\n好\thao\t80000\n吗\tma\t70000\n").unwrap());
    engine.set_input("nihooma");
    assert!(engine.query().unwrap().correction.is_some());
    engine.set_strict_pinyin(true);
    assert!(engine.query().unwrap().correction.is_none());
    assert!(!texts_of(&engine).contains(&"你好吗".to_owned()));
    engine.set_strict_pinyin(false);
    assert!(engine.query().unwrap().correction.is_some());
}

#[test]
fn strict_mode_overrides_fuzzy_without_losing_the_preference() {
    let mut engine =
        Engine::new(Dictionary::parse("知道\tzhi dao\t5000\n自导\tzi dao\t1000\n").unwrap());
    engine.set_fuzzy(FuzzyRules {
        z_zh: true,
        ..FuzzyRules::default()
    });
    engine.set_input("zidao");
    engine.set_strict_pinyin(true);
    assert!(!texts_of(&engine).contains(&"知道".to_owned()));
    engine.set_strict_pinyin(false);
    assert!(texts_of(&engine).contains(&"知道".to_owned()));
}

#[test]
fn cloud_words_obey_the_same_complete_syllables() {
    let mut engine = Engine::new(dictionary());
    engine.set_strict_pinyin(true);
    engine.set_input("liuchu");
    let mut words: Vec<_> = [("流出", "chu"), ("流传", "chuan"), ("流畅", "chang")]
        .into_iter()
        .map(|(text, last)| CloudWord {
            reading: None,
            text: text.to_owned(),
            syllables: vec!["liu".to_owned(), last.to_owned()],
        })
        .collect();
    engine.validate_cloud_words(&mut words);
    assert_eq!(
        words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
        ["流出"]
    );
}

#[test]
fn other_input_schemes_and_english_keep_their_candidates() {
    let mut engine = engine().with_english(WordList::parse("hello\n").unwrap());
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_input("kd");
    let before = texts_of(&engine);
    engine.set_strict_pinyin(true);
    assert_eq!(texts_of(&engine), before);
    engine.set_shuangpin(None);
    engine.set_zhuyin_mode(true);
    assert!(!engine.strict_pinyin_active());
    engine.set_zhuyin_mode(false);
    engine.set_english_mode(true);
    engine.set_input("hell");
    assert!(texts_of(&engine).contains(&"hello".to_owned()));
}
