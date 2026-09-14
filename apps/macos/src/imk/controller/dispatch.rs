//! IMK 按键分发：快捷键优先，其余保持既有输入行为。

use super::*;

impl QingjianInputController {
    /// 一个按键事件的分发：只管按下；Cmd / Ctrl 组合除 Cmd+左右外一律交给应用；命令键映射成选择器；其余按字符当文本。
    pub(super) fn dispatch_event(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        if event.r#type() != NSEventType::KeyDown {
            return false;
        }
        let flags = event.modifierFlags();
        let (command, control, option, shift) = (
            flags.contains(NSEventModifierFlags::Command),
            flags.contains(NSEventModifierFlags::Control),
            flags.contains(NSEventModifierFlags::Option),
            flags.contains(NSEventModifierFlags::Shift),
        );
        if self.handle_punctuation_shortcut(event, client) {
            return true;
        }
        let key = event.keyCode();
        let pressed = Modifiers {
            option,
            shift,
            control,
            command,
        };
        // 提示在显示：敲任何键先收掉，键照常处理
        host::with(|h| h.clear_notice());
        // 翻译选中文字进行中：回车 / 空格 / 1 接受，Esc 放弃，其他键放弃后照常交给应用
        if host::with(|h| h.translation.is_some()).unwrap_or(false) {
            return self.handle_translation_review(key, client);
        }
        // 翻译快捷键（不在组句中）：读应用里的选区，交给云端
        let typed = event
            .charactersByApplyingModifiers(NSEventModifierFlags::empty())
            .map(|c| c.to_string().to_ascii_lowercase());
        let combo = host::with(|h| h.translate_keys).unwrap_or_default();
        if pressed == combo.modifiers
            && typed.as_deref().and_then(|t| t.chars().next()) == Some(combo.key)
            && !host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false)
        {
            return self.translate_selection(client);
        }
        // 修饰键 + 数字：按配置的两组组合上屏第一 / 第二个译词（缺省 ⌥ 与 ⇧⌥）、删候选（缺省 ⇧）。
        // 只在组句中认：不在组句时 ⇧4 就是 `$`，得走下面的标点转换（中文模式出 ￥、⇧6 出 ……、⇧1 出 ！），
        // 以前在这里被截走后原样还给应用，全角转换就没机会做了。
        // 表达式模式（`v2^3`）里 ⇧+数字打的是 `^ * ( )`，不当快捷键
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        if composing
            && !expression
            && !pressed.is_empty()
            && let Some(digit) = digit_key(key)
        {
            let (first, second) = host::with(|h| h.translation_keys).unwrap_or_default();
            if pressed == first {
                return self.handle_translation_key(digit, 0, client);
            }
            if pressed == second {
                return self.handle_translation_key(digit, 1, client);
            }
            if pressed == host::with(|h| h.delete_keys).unwrap_or_default() {
                return self.handle_delete_key(digit, client);
            }
        }
        let selector = match key {
            36 | 76 => Some(sel!(insertNewline:)),
            48 if shift => Some(sel!(insertBacktab:)),
            48 => Some(sel!(insertTab:)),
            51 if option => Some(sel!(deleteWordBackward:)),
            51 if command => Some(sel!(deleteToBeginningOfLine:)),
            51 => Some(sel!(deleteBackward:)),
            117 => Some(sel!(deleteForward:)),
            53 => Some(sel!(cancelOperation:)),
            126 => Some(sel!(moveUp:)),
            125 => Some(sel!(moveDown:)),
            123 if command => Some(sel!(moveToLeftEndOfLine:)),
            124 if command => Some(sel!(moveToRightEndOfLine:)),
            123 if option => Some(sel!(moveWordLeft:)),
            124 if option => Some(sel!(moveWordRight:)),
            123 => Some(sel!(moveLeft:)),
            124 => Some(sel!(moveRight:)),
            116 => Some(sel!(pageUp:)),
            121 => Some(sel!(pageDown:)),
            115 => Some(sel!(moveToBeginningOfLine:)),
            119 => Some(sel!(moveToEndOfLine:)),
            _ => None,
        };
        if let Some(selector) = selector {
            return self.handle_command(selector, client);
        }
        if command || control {
            return false;
        }
        match event.characters() {
            Some(text) if !text.is_empty() => self.handle_text(&text.to_string(), client),
            _ => false,
        }
    }
}
