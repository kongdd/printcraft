//! Small, view-only tab operations. Documents and their reading state move together.

use std::path::{Path, PathBuf};

use crate::PrintCraftApp;

fn file_key(path: &str) -> PathBuf {
    let path = Path::new(path);
    std::fs::canonicalize(path).or_else(|_| std::path::absolute(path)).unwrap_or_else(|_| path.to_path_buf())
}

impl PrintCraftApp {
    /// Activate an already-open file without re-reading or replacing its contents.
    pub(crate) fn focus_open_file(&mut self, path: &str) -> bool {
        // Avoid filesystem access for the common case, especially on a network drive.
        let existing = self.views.iter().position(|v| self.session.get(v.id).and_then(|d| d.path.as_deref()) == Some(path)).or_else(|| {
            let key = file_key(path);
            self.views.iter().position(|v| self.session.get(v.id).and_then(|d| d.path.as_deref()).is_some_and(|p| file_key(p) == key))
        });
        if let Some(index) = existing {
            self.active = Some(index);
            self.tab_reveal = true;
            if let Some(ctx) = &self.ctx {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.request_repaint();
            }
            true
        } else {
            false
        }
    }

    /// Select a tab in strip order, wrapping at either end; Home is not a document tab.
    pub fn step_tab(&mut self, forward: bool) {
        let n = self.views.len();
        if n == 0 || self.close_request.is_some() {
            return;
        }
        self.active = Some(match self.active.filter(|&i| i < n) {
            None => {
                if forward {
                    0
                } else {
                    n - 1
                }
            }
            Some(i) if forward => {
                if i + 1 == n {
                    0
                } else {
                    i + 1
                }
            }
            Some(0) => n - 1,
            Some(i) => i - 1,
        });
        self.tab_reveal = true;
    }

    /// Reorder tabs while preserving the active document, its view, and unsaved edits.
    /// Reordering is blocked during the save prompt, which refers to a tab index.
    pub fn move_tab(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.views.len() || to >= self.views.len() {
            return Err("tab index is outside the open documents".into());
        }
        if self.close_request.is_some() {
            return Err("finish the save-changes prompt before moving tabs".into());
        }
        if from != to {
            let active_id = self.active.and_then(|i| self.views.get(i)).map(|v| v.id);
            let view = self.views.remove(from);
            self.views.insert(to, view);
            self.active = active_id.and_then(|id| self.views.iter().position(|v| v.id == id));
        }
        Ok(())
    }

    pub(crate) fn tab_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        // SumatraPDF's simple strip-order mode. Ctrl+Tab also works on macOS.
        for (modifiers, forward) in [(Modifiers::CTRL | Modifiers::SHIFT, false), (Modifiers::CTRL, true)] {
            if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(modifiers, Key::Tab))) {
                self.step_tab(forward);
            }
        }
    }
}
