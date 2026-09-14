//! 用真实 AppKit 候选窗口验证缩放与屏幕边界，并导出人工审阅用 PNG。
//! 运行：cargo run -p qingjian-macos --example candidate_preview -- target/verification

#[allow(dead_code)]
#[path = "../src/candidates/mod.rs"]
mod candidates;

use candidates::{CandidateWindow, Frame, Preedit, Row};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBitmapImageFileType, NSEvent,
    NSEventModifierFlags, NSEventType, NSScreen,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};
use qingjian_core::{Candidate, CandidateKind, Language, PartOfSpeech, Sense, Translation};
use qingjian_platform::{CandidateScale, LayoutMode};

fn sample() -> Frame {
    let rows = [
        ("你好", "hello"),
        ("您好", "hello (polite)"),
        ("你好吗", "how are you"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (text, gloss))| {
        let mut row = Row::from_candidate(
            index,
            &Candidate {
                text: text.to_owned(),
                kind: CandidateKind::Chinese,
                syllables: vec![],
                reading: None,
                translation: Some(Translation::new(
                    Language::English,
                    vec![Sense {
                        part_of_speech: Some(PartOfSpeech::Noun),
                        text: gloss.to_owned(),
                        reading: None,
                        fresh: false,
                    }],
                )),
            },
        );
        row.cloud = index == 2;
        row
    })
    .collect();
    Frame {
        preedit: Preedit::plain("ni'hao", 6),
        rows,
        highlighted: 1,
        footer: Some("1/3".to_owned()),
        ..Frame::default()
    }
}

fn close(actual: f64, expected: f64) {
    // WindowServer 把窗口尺寸取整到屏幕像素，允许不足一个点的量化误差。
    assert!((actual - expected).abs() < 1.0, "{actual} != {expected}");
}

fn main() {
    let output = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let mtm = MainThreadMarker::new().expect("preview runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let screen = NSScreen::mainScreen(mtm)
        .expect("a graphical macOS session is required")
        .visibleFrame();
    let anchor = NSRect::new(
        NSPoint::new(
            screen.origin.x + screen.size.width - 2.0,
            screen.origin.y + 2.0,
        ),
        NSSize::new(1.0, 16.0),
    );
    let mut window = CandidateWindow::new(mtm);
    let frame = sample();
    for (label, layout) in [
        ("vertical", LayoutMode::Vertical),
        ("horizontal", LayoutMode::Horizontal),
    ] {
        window.set_layout(layout);
        window.set_scale(CandidateScale::try_from(100).unwrap());
        window.show(frame.clone(), anchor);
        let panel = app.windows().objectAtIndex(0);
        let view = panel.contentView().unwrap();
        let logical = view.bounds().size;
        for scale in CandidateScale::ALL {
            window.set_scale(scale);
            let physical = panel.frame();
            close(physical.size.width, logical.width * scale.factor());
            close(physical.size.height, logical.height * scale.factor());
            close(view.bounds().size.width, logical.width);
            close(view.bounds().size.height, logical.height);
            assert!(physical.origin.x >= screen.origin.x);
            assert!(physical.origin.y >= screen.origin.y);
            assert!(
                physical.origin.x + physical.size.width
                    <= screen.origin.x + screen.size.width + 0.1
            );
            assert!(
                physical.origin.y + physical.size.height
                    <= screen.origin.y + screen.size.height + 0.1
            );
            let bounds = view.bounds();
            let bitmap = view.bitmapImageRepForCachingDisplayInRect(bounds).unwrap();
            view.cacheDisplayInRect_toBitmapImageRep(bounds, &bitmap);
            // SAFETY: PNG 使用默认编码参数，空字典没有不匹配的属性值。
            let png = unsafe {
                bitmap.representationUsingType_properties(
                    NSBitmapImageFileType::PNG,
                    &NSDictionary::new(),
                )
            }
            .unwrap();
            let path = output.join(format!("candidate-{}-{label}.png", scale.percent()));
            assert!(png.writeToFile_atomically(&NSString::from_str(&path.to_string_lossy()), true));
            println!(
                "{label} {}%: {:?}, bounds {:?}; edge placement OK",
                scale.percent(),
                physical,
                view.bounds()
            );
        }
    }
    window.hide();
    // 这是 AppKit 原生事件，不向任何应用发送按键；验证 Shift 下基础句号的读取方式。
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
        NSEventType::KeyDown, NSPoint::ZERO, NSEventModifierFlags::Option | NSEventModifierFlags::Shift,
        0.0, 0, None, &NSString::from_str("˘"), &NSString::from_str(">"), false, 47,
    ).unwrap();
    assert_eq!(
        event
            .charactersByApplyingModifiers(NSEventModifierFlags::empty())
            .unwrap()
            .to_string(),
        "."
    );
    println!("Option+Shift+period normalization OK");
}
