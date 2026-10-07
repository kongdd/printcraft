//! Tab identity, duplicate opens, input gestures, and overflow in the real egui shell.

use egui_kittest::{Harness, kittest::Queryable};
use printcraft_ui_egui::PrintCraftApp;

const PDF: &[u8] = b"%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >> endobj\ntrailer << /Root 1 0 R >>\n%%EOF";

fn app() -> PrintCraftApp {
    let mut app = PrintCraftApp::new();
    for name in ["alpha.pdf", "beta.pdf", "gamma.pdf"] {
        app.open_bytes(name, None, PDF.to_vec()).unwrap();
    }
    app
}

fn harness(n: usize, width: f32) -> Harness<'static, PrintCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(width, 700.0)).build_eframe(move |_| {
        let mut app = PrintCraftApp::new();
        for i in 0..n {
            app.open_bytes(&format!("document-{i}.pdf"), None, PDF.to_vec()).unwrap();
        }
        app
    });
    h.run_steps(12);
    h
}

#[test]
fn closing_background_tabs_preserves_active_document() {
    let mut app = app();
    let active = app.active_ids().unwrap().1;
    app.close_tab(0);
    assert_eq!(app.active_ids().unwrap().1, active);
    assert_eq!(app.active, Some(1));
    app.close_tab(1);
    assert_eq!(app.active, Some(0));
    app.close_tab(0);
    assert_eq!(app.active, None);
    app.step_tab(true);
    assert_eq!(app.active, None);
}

#[test]
fn moving_tabs_keeps_reading_state_and_active_identity() {
    let mut app = app();
    let active = app.active_ids().unwrap().1;
    app.views[2].set_zoom(1.75);
    app.move_tab(2, 0).unwrap();
    assert_eq!(app.active, Some(0));
    assert_eq!(app.active_ids().unwrap().1, active);
    assert_eq!(app.views[0].zoom, 1.75);
    app.move_tab(2, 1).unwrap();
    assert_eq!(app.active_ids().unwrap().1, active);
    assert!(app.move_tab(usize::MAX, 0).is_err());
    app.active = None;
    app.move_tab(0, 2).unwrap();
    assert_eq!(app.active, None);
}

#[test]
fn duplicate_path_focuses_without_replacing_unsaved_work() {
    let mut app = app();
    let path = std::env::temp_dir().join(format!("printcraft-tabs-{}.pdf", std::process::id()));
    let path = path.to_string_lossy().into_owned();
    app.open_bytes("original.pdf", Some(path.clone()), PDF.to_vec()).unwrap();
    let id = app.active_ids().unwrap().1;
    app.views[3].set_zoom(2.0);
    assert!(app.apply_edit(printcraft_engine::Edit::RotatePages { pages: vec![0], degrees: 90 }));
    app.active = Some(0);
    // No file exists: focusing must happen before attempting a disk read.
    app.open_path(&path);
    assert_eq!(app.views.len(), 4);
    assert_eq!(app.active_ids().unwrap().1, id);
    assert_eq!(app.views[3].zoom, 2.0);
    assert!(app.session.get(id).unwrap().dirty);
    // A changed or malformed byte buffer also must not replace the open document.
    app.open_bytes("same.pdf", Some(path), b"not a pdf".to_vec()).unwrap();
    assert_eq!(app.views.len(), 4);
    assert!(app.session.get(id).unwrap().dirty);
}

#[cfg(unix)]
#[test]
fn symlink_and_dot_paths_share_a_tab() {
    let dir = std::env::temp_dir().join(format!("printcraft-tabs-alias-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("file.pdf");
    let alias = dir.join("alias.pdf");
    std::fs::write(&file, PDF).unwrap();
    std::os::unix::fs::symlink(&file, &alias).unwrap();
    let mut app = PrintCraftApp::new();
    app.open_path(&file.to_string_lossy());
    let id = app.active_ids().unwrap().1;
    app.open_path(&alias.to_string_lossy());
    app.open_path(&dir.join(".").join("file.pdf").to_string_lossy());
    assert_eq!(app.views.len(), 1);
    assert_eq!(app.active_ids().unwrap().1, id);
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn tab_options_and_commands_work_without_a_window() {
    let mut app = app();
    app.set_option("tab", "1").unwrap();
    assert!(app.execute("view.previous_tab"));
    assert_eq!(app.active, Some(2));
    assert!(app.execute("view.next_tab"));
    assert_eq!(app.active, Some(0));
    app.set_option("tab_move", "1:3").unwrap();
    assert_eq!(app.active, Some(2));
    for value in ["0", "4", "-1", "no"] {
        assert!(app.set_option("tab", value).is_err());
        assert_eq!(app.active, Some(2));
    }
    assert!(app.set_option("tab_move", "0:1").is_err());
}

#[test]
fn save_prompt_prevents_reordering_and_middle_close_keeps_edits() {
    let mut h = harness(3, 1200.0);
    h.state_mut().set_option("tab", "1").unwrap();
    assert!(h.state_mut().apply_edit(printcraft_engine::Edit::RotatePages { pages: vec![0], degrees: 90 }));
    h.run_steps(12);
    let p = h.get_by_label("document-0.pdf (edited)").rect().center();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Middle, pressed, modifiers: egui::Modifiers::NONE });
        h.step();
    }
    assert_eq!(h.state().views.len(), 3);
    assert!(h.state().close_request.is_some());
    assert!(h.state_mut().move_tab(0, 2).is_err());
}

#[test]
fn middle_click_closes_and_keyboard_switches_in_strip_order() {
    let mut h = harness(3, 1200.0);
    let active = h.state().active_ids().unwrap().1;
    let p = h.get_by_label("document-0.pdf").rect().center();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Middle, pressed, modifiers: egui::Modifiers::NONE });
        h.step();
    }
    assert_eq!(h.state().views.len(), 2);
    assert_eq!(h.state().active_ids().unwrap().1, active);
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().active, Some(0));
    h.key_press_modifiers(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().active, Some(1));
}

#[test]
fn drag_reorders_tabs_without_changing_the_active_document() {
    let mut h = harness(3, 1200.0);
    let active = h.state().active_ids().unwrap().1;
    let first = h.state().views[0].id;
    let from = h.get_by_label("document-0.pdf").rect().center();
    let to = h.get_by_label("document-2.pdf").rect().center();
    h.hover_at(from);
    h.step();
    h.drag_at(from);
    h.step();
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.step();
    }
    h.drop_at(to);
    h.run_steps(4);
    assert_eq!(h.state().views[2].id, first);
    assert_eq!(h.state().active_ids().unwrap().1, active);
}

#[test]
fn selecting_the_same_tab_reveals_it_after_manual_scrolling() {
    let mut h = harness(30, 900.0);
    h.state_mut().set_option("tab", "1").unwrap();
    h.run_steps(4);
    h.hover_at(egui::pos2(300.0, 20.0));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-3000.0, 0.0),
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    h.run_steps(20);
    assert_eq!(h.state().active, Some(0));
    assert!(h.query_all_by_label("document-0.pdf").count() == 0 || h.get_by_label("document-0.pdf").rect().left() < 0.0);
    h.state_mut().set_option("tab", "1").unwrap();
    h.run_steps(4);
    let rect = h.get_by_label("document-0.pdf").rect();
    assert!(rect.left() >= 0.0 && rect.right() <= 900.0, "revealed tab {rect:?}");
}

#[test]
fn overflow_keeps_controls_visible_and_lists_all_tabs() {
    let mut h = harness(30, 900.0);
    let active = h.get_by_label("document-29.pdf").rect();
    assert!(active.left() >= 0.0 && active.right() <= 900.0, "active tab {active:?}");
    h.get_by_label("Open tabs").click();
    h.run_steps(3);
    assert!(h.query_all_by_label("document-0.pdf").count() > 0);
    h.get_all_by_label("document-0.pdf").last().unwrap().click();
    h.run_steps(12);
    assert_eq!(h.state().active, Some(0));
    let first = h.get_by_label("document-0.pdf").rect();
    assert!(first.left() >= 0.0 && first.right() <= 900.0, "selected tab {first:?}");
    if let Ok(path) = std::env::var("PRINTCRAFT_TABS_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
}

#[test]
fn tab_search_matches_paths_and_keeps_identical_names_distinct() {
    let mut app = PrintCraftApp::new();
    for path in ["/research/2025/report.pdf", "/research/2026/report.pdf"] {
        app.open_bytes("report.pdf", Some(path.into()), PDF.to_vec()).unwrap();
    }
    assert_eq!(app.matching_tabs("REPORT"), vec![0, 1]);
    assert_eq!(app.matching_tabs("2026 report"), vec![1]);
    assert!(app.matching_tabs("missing").is_empty());
    assert_eq!(app.matching_tabs(""), vec![0, 1]);
}

#[test]
fn numbered_shortcuts_select_first_eighth_and_last_tabs() {
    let mut h = harness(30, 900.0);
    for (key, expected) in [(egui::Key::Num1, 0), (egui::Key::Num8, 7), (egui::Key::Num9, 29)] {
        h.key_press_modifiers(egui::Modifiers::ALT, key);
        h.run_steps(4);
        assert_eq!(h.state().active, Some(expected));
        let rect = h.get_by_label(&format!("document-{expected}.pdf")).rect();
        assert!(rect.left() >= 0.0 && rect.right() <= 900.0);
    }
}

#[test]
fn palette_finds_overflow_tabs_and_navigates_beyond_twelve_results() {
    let mut h = harness(40, 900.0);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    h.run_steps(3);
    h.event(egui::Event::Text("@document-23".into()));
    h.run_steps(4);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert_eq!(h.state().active, Some(23));
    assert!(!h.state().palette_open);
    h.state_mut().set_option("palette", "@missing").unwrap();
    h.run_steps(3);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(h.state().palette_open);
    assert_eq!(h.state().active, Some(23));
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    h.state_mut().set_option("palette", "@").unwrap();
    h.run_steps(3);
    let result = h.get_all_by_label("document-8.pdf").last().unwrap().rect();
    assert!(result.bottom() < 520.0, "tab list must show several results: {result:?}");
    if let Ok(path) = std::env::var("PRINTCRAFT_MANY_TABS_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    for _ in 0..25 {
        h.key_press(egui::Key::ArrowDown);
        h.run_steps(2);
    }
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert_eq!(h.state().active, Some(25));
}

#[test]
fn tab_strip_shrinks_before_overflow_and_dropdown_filters() {
    let mut h = harness(4, 900.0);
    let width = h.get_by_label("document-3.pdf").rect().width();
    assert!((110.0..220.0).contains(&width));
    let first = h.get_by_label("document-0.pdf").rect();
    assert!(first.left() >= 0.0 && first.right() <= 900.0);
    h.get_by_label("Open tabs").click();
    h.run_steps(3);
    h.event(egui::Event::Text("document-1".into()));
    h.run_steps(3);
    // Strip contributes one label; the filtered list contributes only its match.
    assert_eq!(h.query_all_by_label("document-0.pdf").count(), 1);
    assert_eq!(h.query_all_by_label("document-1.pdf").count(), 2);
    h.get_all_by_label("document-1.pdf").last().unwrap().click();
    h.run_steps(4);
    assert_eq!(h.state().active, Some(1));
}
