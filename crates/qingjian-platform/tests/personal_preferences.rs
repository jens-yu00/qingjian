//! 个人候选显示与标点快捷键的配置契约。

use qingjian_platform::{Config, KeyCombo};

#[test]
fn old_configs_get_readable_scale_and_the_punctuation_shortcut() {
    let config: Config = toml::from_str("[general]\npage_size = 5\n").unwrap();
    let saved = toml::to_string(&config).unwrap();
    assert!(saved.contains("candidate_scale = 150"));
    assert!(saved.contains("toggle_punctuation = \"shift+option+.\""));
    assert_eq!(config.general.page_size(), 5);
}

#[test]
fn scale_and_punctuation_round_trip_without_changing_other_preferences() {
    let config: Config = toml::from_str(
        "[general]\ncandidate_scale = 200\nfull_width_punctuation = false\nlayout = 'horizontal'\n",
    )
    .unwrap();
    let saved = toml::to_string(&config).unwrap();
    assert!(saved.contains("candidate_scale = 200"));
    assert!(!config.general.full_width_punctuation);
    assert_eq!(toml::from_str::<Config>(&saved).unwrap(), config);
}

#[test]
fn invalid_scale_is_rejected_instead_of_reaching_the_renderer() {
    for value in ["0", "99", "151", "251", "-1", "nan", "inf", "'large'"] {
        assert!(
            toml::from_str::<Config>(&format!("[general]\ncandidate_scale = {value}\n")).is_err(),
            "accepted {value}"
        );
    }
}

#[test]
fn period_shortcut_round_trips_with_both_modifiers() {
    let combo: KeyCombo = "option+shift+.".parse().unwrap();
    assert!(combo.modifiers.option && combo.modifiers.shift);
    assert!(!combo.modifiers.command && !combo.modifiers.control);
    assert_eq!(combo.key, '.');
    assert_eq!(combo.key_string().parse::<KeyCombo>().unwrap(), combo);
}

#[test]
fn strict_pinyin_persists_and_legacy_configs_keep_their_behavior() {
    let old: Config = toml::from_str("[general]\ncandidate_scale = 150\n").unwrap();
    assert!(!old.general.strict_pinyin);
    let mut enabled = old;
    enabled.general.strict_pinyin = true;
    let saved = toml::to_string(&enabled).unwrap();
    assert_eq!(toml::from_str::<Config>(&saved).unwrap(), enabled);
}
