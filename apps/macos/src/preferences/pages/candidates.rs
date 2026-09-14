//! 「候选窗口」页：外观、排布、拼音显示位置。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSPopUpButton;
use qingjian_platform::{CandidateScale, Config, LayoutMode, PreeditMode, ThemeMode};

use crate::preferences::controls::{note, row_popup, select};
use crate::preferences::layout::Layout;
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

pub struct CandidatesPage {
    /// 外观：跟随系统 / 浅色 / 深色。
    theme: Retained<NSPopUpButton>,

    /// 整体缩放档位。
    scale: Retained<NSPopUpButton>,

    /// 竖排 / 横排。
    layout_mode: Retained<NSPopUpButton>,

    /// 拼音显示位置。
    preedit: Retained<NSPopUpButton>,
}

impl CandidatesPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        let theme_titles: Vec<String> = ThemeMode::ALL
            .iter()
            .map(|t| t.label().to_owned())
            .collect();
        let theme = row_popup(layout, mtm, "外观", &theme_titles, Setting::Theme, target);
        let scale_titles: Vec<String> = CandidateScale::ALL
            .iter()
            .map(|s| format!("{}%", s.percent()))
            .collect();
        let scale = row_popup(
            layout,
            mtm,
            "整体缩放",
            &scale_titles,
            Setting::CandidateScale,
            target,
        );
        note(
            layout,
            mtm,
            "候选字、拼音、译文、序号和间距一起放大；不改变应用内的行内拼音字号。",
        );
        let layout_titles: Vec<String> = LayoutMode::ALL
            .iter()
            .map(|l| l.label().to_owned())
            .collect();
        let layout_mode = row_popup(layout, mtm, "排布", &layout_titles, Setting::Layout, target);
        note(layout, mtm, "横排时只给高亮的候选显示译词。");
        let preedit_titles: Vec<String> = PreeditMode::ALL
            .iter()
            .map(|p| p.label().to_owned())
            .collect();
        let preedit = row_popup(
            layout,
            mtm,
            "拼音显示",
            &preedit_titles,
            Setting::Preedit,
            target,
        );
        note(
            layout,
            mtm,
            "「只在候选窗口」时正在敲的拼音不显示在应用里，终端或行内拼音显示不正常的应用可以选它。",
        );
        Self {
            theme,
            scale,
            layout_mode,
            preedit,
        }
    }

    pub fn sync(&self, config: &Config) {
        let general = &config.general;
        select(
            &self.scale,
            CandidateScale::ALL
                .iter()
                .position(|s| *s == general.candidate_scale),
        );
        select(
            &self.theme,
            ThemeMode::ALL.iter().position(|t| *t == general.theme),
        );
        select(
            &self.layout_mode,
            LayoutMode::ALL.iter().position(|l| *l == general.layout),
        );
        select(
            &self.preedit,
            PreeditMode::ALL.iter().position(|p| *p == general.preedit),
        );
    }
}
