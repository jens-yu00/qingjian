//! 中文模式的标点偏好切换，不改变当前组句或候选选择。

use super::*;

impl QingjianInputController {
    pub(super) fn handle_punctuation_shortcut(
        &self,
        event: &NSEvent,
        client: TextClient<'_>,
    ) -> bool {
        let flags = event.modifierFlags();
        if flags.contains(NSEventModifierFlags::CapsLock) {
            return false;
        }
        let pressed = Modifiers {
            option: flags.contains(NSEventModifierFlags::Option),
            shift: flags.contains(NSEventModifierFlags::Shift),
            control: flags.contains(NSEventModifierFlags::Control),
            command: flags.contains(NSEventModifierFlags::Command),
        };
        let Some(combo) = host::with(|h| h.settings.config().shortcut.toggle_punctuation) else {
            return false;
        };
        if pressed != combo.modifiers {
            return false;
        }
        // charactersIgnoringModifiers 仍保留 Shift，会把句号变成 >；这里明确取无修饰键的字符。
        let key = event
            .charactersByApplyingModifiers(NSEventModifierFlags::empty())
            .and_then(|text| text.to_string().chars().next())
            .map(|c| c.to_ascii_lowercase());
        if key != Some(combo.key) {
            return false;
        }
        // 长按只切一次，同时吞掉重复事件，避免交给应用插入符号。
        if event.isARepeat() {
            return true;
        }
        let anchor = client.caret_rect();
        host::with(|h| {
            h.reload_config_if_changed();
            let enabled = !h.settings.config().general.full_width_punctuation;
            if !h
                .settings
                .set_bool("general", "full_width_punctuation", enabled)
            {
                h.preferences
                    .set_status("标点设置保存失败，仍使用原来的标点模式。");
                return;
            }
            h.apply_config(false);
            let message = if enabled {
                "中文标点 ，。"
            } else {
                "英文标点 ,."
            };
            if h.engine.composition().is_empty() && h.translation.is_none() {
                h.clear_notice();
                h.show_notice(message, anchor);
            } else {
                // 组句中只改窗口提示，不能用 show_notice 重置候选会话。
                h.status = Some(message.to_owned());
                h.render();
            }
        });
        true
    }
}
