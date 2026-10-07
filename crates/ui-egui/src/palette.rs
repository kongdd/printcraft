//! ⌘K command palette: every registered command (with its shortcut) and every tool in the
//! catalogue, with a fuzzy-ish substring match.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};
use printcraft_engine::catalog::{Availability, TOOL_GROUPS};

use crate::theme::{self, Tokens};
use crate::{LeftPanel, PrintCraftApp, icons};

struct Hit {
    /// The tool panel to open (tools and catalogue items); `None` for plain commands.
    group: Option<&'static str>,
    label: String,
    detail: String,
    icon: &'static str,
    command: Option<&'static str>,
    ready: bool,
}

fn score(hay: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let h = hay.to_lowercase();
    if let Some(p) = h.find(needle) {
        return Some(p);
    }
    // Subsequence match as a fallback.
    let mut it = h.chars();
    needle.chars().all(|c| it.any(|x| x == c)).then_some(100)
}

pub fn show(app: &mut PrintCraftApp, ctx: &egui::Context) {
    if !app.palette_open {
        return;
    }
    let t = Tokens::get(ctx);
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.palette_open = false;
        return;
    }
    if app.palette_query.trim_start().starts_with('@') {
        show_tabs(app, ctx);
        return;
    }
    let q = app.palette_query.trim().to_lowercase();
    let mut hits: Vec<(usize, Hit)> = Vec::new();
    let mac = cfg!(target_os = "macos") || cfg!(target_arch = "wasm32");
    let active = app.active_ids().map(|(_, id)| id);
    for spec in printcraft_engine::commands::COMMANDS {
        let label = printcraft_engine::commands::current_label(spec, &app.session, active);
        if let Some(s) = score(&label, &q).or_else(|| score(spec.id, &q).map(|s| s + 50)) {
            hits.push((
                s,
                Hit {
                    group: None,
                    label,
                    detail: spec.shortcut.map(|k| k.label(mac)).unwrap_or_else(|| spec.menu.unwrap_or("Command").to_string()),
                    icon: spec.icon,
                    command: Some(spec.id),
                    ready: app.command_enabled(spec),
                },
            ));
        }
    }
    for g in TOOL_GROUPS {
        if let Some(s) = score(g.label, &q) {
            hits.push((
                s,
                Hit {
                    group: Some(g.id),
                    label: g.label.to_string(),
                    detail: "Tool".into(),
                    icon: g.icon,
                    command: None,
                    ready: g.availability == Availability::Ready,
                },
            ));
        }
        for sec in g.sections {
            for i in sec.items {
                if printcraft_engine::commands::command(i.command).is_some() {
                    continue; // listed above as a command
                }
                if let Some(s) = score(i.label, &q).or_else(|| score(i.command, &q).map(|s| s + 50)) {
                    hits.push((
                        s + 1,
                        Hit {
                            group: Some(g.id),
                            label: i.label.to_string(),
                            detail: g.label.into(),
                            icon: i.icon,
                            command: Some(i.command),
                            ready: i.availability == Availability::Ready,
                        },
                    ));
                }
            }
        }
    }
    hits.sort_by_key(|(s, h)| (*s, !h.ready));
    hits.truncate(12);

    let screen = ctx.content_rect();
    let mut chosen: Option<(Option<&'static str>, Option<&'static str>)> = None;
    egui::Area::new(egui::Id::new("palette"))
        .order(egui::Order::Foreground)
        .pivot(Align2::CENTER_TOP)
        .fixed_pos(egui::pos2(screen.center().x, screen.top() + 96.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                ui.set_width(560.0);
                ui.horizontal(|ui| {
                    ui.add(icons::image("search", 18.0, t.text_muted));
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut app.palette_query)
                            .id(egui::Id::new("palette-query"))
                            .hint_text("Search commands, or @ for tabs…")
                            .frame(egui::Frame::NONE)
                            .font(theme::regular(15.0))
                            .desired_width(f32::INFINITY),
                    );
                    // Enter runs the top hit. The field keeps focus (requested every frame), so
                    // check while it is focused as well as when focus is lost.
                    if (r.has_focus() || r.lost_focus())
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && let Some((_, h)) = hits.first()
                    {
                        chosen = Some((h.command, h.group));
                    }
                    r.request_focus();
                });
                ui.separator();
                for (_, h) in &hits {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(6), t.hover);
                    }
                    icons::paint(ui, Rect::from_min_size(rect.min + vec2(8.0, 9.0), vec2(18.0, 18.0)), h.icon, 17.0, t.icon);
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, h.ready, &h.label));
                    let fg = if h.ready { t.text } else { t.text_faint };
                    ui.painter().text(rect.left_center() + vec2(36.0, 0.0), Align2::LEFT_CENTER, &h.label, theme::regular(13.5), fg);
                    ui.painter().text(rect.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, &h.detail, theme::regular(12.0), t.text_faint);
                    if resp.clicked() {
                        chosen = Some((h.command, h.group));
                    }
                }
                if hits.is_empty() {
                    ui.label(egui::RichText::new("No matching tools").color(t.text_muted));
                }
                ui.add_space(2.0);
                let _ = Stroke::NONE;
            });
        });
    if let Some((command, group)) = chosen {
        app.palette_open = false;
        app.palette_query.clear();
        if let Some(g) = group {
            app.left_open = true;
            app.left = LeftPanel::Tool(g);
        }
        if let Some(c) = command {
            app.run_command(c);
        }
    }
}

/// Sumatra-style @ tab search, using the same palette and stable document identities.
fn show_tabs(app: &mut PrintCraftApp, ctx: &egui::Context) {
    let selection_id = egui::Id::new("palette-tab-selection");
    let mut selected = ctx.data_mut(|d| d.get_temp::<usize>(selection_id)).unwrap_or(0);
    let mut chosen = None;
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("palette"))
        .order(egui::Order::Foreground)
        .pivot(Align2::CENTER_TOP)
        .fixed_pos(egui::pos2(screen.center().x, screen.top() + 96.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                ui.set_width(560.0_f32.min(screen.width() - 40.0).max(100.0));
                // Reserve list navigation before TextEdit can treat these as cursor keys.
                let down = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.palette_query)
                        .id(egui::Id::new("palette-query"))
                        .hint_text("@filename or path")
                        .desired_width(f32::INFINITY),
                );
                response.request_focus();
                if response.changed() {
                    selected = 0;
                }
                let query = app.palette_query.trim_start().strip_prefix('@').unwrap_or("");
                let matches = app.matching_tabs(query);
                selected = selected.min(matches.len().saturating_sub(1));
                if !matches.is_empty() {
                    if down {
                        selected = (selected + 1) % matches.len();
                    }
                    if up {
                        selected = if selected == 0 { matches.len() - 1 } else { selected - 1 };
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        chosen = matches.get(selected).copied();
                    }
                }
                ui.label(format!("{} matching / {} open tabs", matches.len(), app.views.len()));
                ui.separator();
                egui::ScrollArea::vertical().min_scrolled_height((screen.height() - 220.0).clamp(100.0, 360.0)).max_height(360.0).show(ui, |ui| {
                    for (row, &index) in matches.iter().enumerate() {
                        let Some(view) = app.views.get(index) else { continue };
                        let Some(doc) = app.session.get(view.id) else { continue };
                        ui.push_id(view.id.0, |ui| {
                            let name = doc.display_name();
                            let label = if doc.dirty { format!("{name} •") } else { name };
                            let response = ui.selectable_label(row == selected, label);
                            if response.clicked() {
                                chosen = Some(index);
                            }
                            if row == selected && (up || down || response.changed()) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            if let Some(path) = &doc.path {
                                ui.add(egui::Label::new(egui::RichText::new(path).small().weak()).truncate()).on_hover_text(path);
                            }
                        });
                    }
                });
                if matches.is_empty() {
                    ui.label("No matching tabs");
                }
            });
        });
    ctx.data_mut(|d| d.insert_temp(selection_id, selected));
    if let Some(index) = chosen.filter(|_| app.close_request.is_none()) {
        app.active = Some(index);
        app.tab_reveal = true;
        app.palette_open = false;
        app.palette_query.clear();
        ctx.data_mut(|d| d.remove::<usize>(selection_id));
    }
}
