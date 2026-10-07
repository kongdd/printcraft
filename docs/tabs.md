# Document tabs

The tab strip stays on one line. Scroll horizontally (or use the mouse wheel over
the strip) when it overflows. Selecting a tab from the **Open tabs** dropdown or
with a shortcut brings it into view.

- Drag a tab onto another visible tab to move it to that position.
- Middle-click a tab, or use its close button, to close it. Unsaved changes still
  require the existing Save / Don't save / Cancel prompt.
- `Ctrl+Tab` / `Ctrl+Shift+Tab` switch in strip order, wrapping at the ends,
  including on macOS. This implements SumatraPDF's simple tab-switch mode, not
  its MRU Smart Tab Switch overlay.
- `Ctrl+PageDown` / `Ctrl+PageUp` switch on Windows/Linux;
  `Cmd+PageDown` / `Cmd+PageUp` switch on macOS. These commands are also in the
  View menu and command palette.
- Closing or moving a background tab preserves the active document and its
  reading state. Closing the active tab selects its right neighbour, or its left
  neighbour when it was last. Closing the final document returns to Home.
- Opening an already-open file focuses its existing tab/window without reading
  it again or replacing unsaved work. Path aliases that canonicalize to the same
  path (including symlinks) are recognized. Anonymous byte documents are not
  deduplicated by filename, since different files can have identical names.

File deduplication is within the running application instance. macOS Finder
events use the same open path. Cross-process single-instance forwarding on
Windows/Linux, hard-link identity, tab tear-off, close-other-tabs, and session
restore are outside this change.

## Opt-in UI automation

Use the existing `--control` channel, without enabling any new listeners:

```text
ui.set {"key":"tab","value":2}
ui.set {"key":"tab_move","value":"2:1"}
ui.command {"id":"view.next_tab"}
ui.command {"id":"view.previous_tab"}
```

Indices are 1-based. `tab_move` preserves the active document and is rejected
while the save-changes prompt is open. `ui.state` reports the updated order and
active document.

## Validation

`crates/ui-egui/tests/tabs.rs` covers duplicate opens, path aliases, preservation
of unsaved work and reading state, background-tab closure, keyboard switching,
middle-click, drag-and-drop, overflow, and the tab list. The control-channel test
in `crates/ui-egui/tests/control.rs` covers selection and reorder through
`ui.set` and switching through `ui.command`.
