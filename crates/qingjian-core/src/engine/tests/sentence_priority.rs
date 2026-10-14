//! 完整词与自动组句竞争时的排序，兼顾歧义切分与未收录表达。

use super::*;

#[test]
fn exact_word_in_another_segmentation_precedes_constructed_sentence() {
    let mut engine = Engine::new(Dictionary::parse(
        "严格\tyan ge\t5000\n沿革\tyan ge\t1000\n样\tyang\t100000\n哦\te\t100000\n严\tyan\t100\n格\tge\t100\n",
    ).unwrap());
    engine.set_strict_pinyin(true);
    engine.set_input("yange");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "严格");
    assert_eq!(items[1].text, "沿革");
    assert!(items.iter().any(|c| c.text == "样哦"));
    assert_eq!(engine.commit(&items[0]), "严格");
    assert!(engine.composition().is_empty());

    // 显式分隔符选择 yang e 时，不得把不同音节边界的 yan ge 当作完整命中。
    engine.set_input("yang'e");
    assert_eq!(texts_of(&engine)[0], "样哦");
    assert!(!texts_of(&engine).contains(&"严格".to_owned()));
}

#[test]
fn text_frequency_cannot_push_a_constructed_ordinal_above_the_exact_word() {
    let mut engine = Engine::new(
        Dictionary::parse(
            "第一个\tdi yi ge\t5000\n的\tdi\t100000\n的\tde\t100000\n一个\tyi ge\t100000\n",
        )
        .unwrap(),
    )
    .with_learner(Box::new(CountingLearner(HashMap::from([
        ("的".to_owned(), 100000),
        ("一个".to_owned(), 100000),
    ]))));
    engine.set_strict_pinyin(true);
    engine.set_input("diyige");
    assert_eq!(texts_of(&engine)[0], "第一个");
    engine.set_input("deyige");
    assert_eq!(texts_of(&engine)[0], "的一个");
}

#[test]
fn unlisted_expression_still_leads_and_commits_as_a_sentence() {
    let mut engine = engine();
    engine.set_strict_pinyin(true);
    engine.set_input("xiangkaifa");
    let first = engine.query().unwrap().candidates.items.remove(0);
    assert_eq!(first.text, "想开发");
    assert_eq!(first.kind, CandidateKind::Sentence);
    assert_eq!(engine.commit(&first), "想开发");
    assert!(engine.composition().is_empty());
}
