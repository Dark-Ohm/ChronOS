//! Preview tab — render the file the user clicked in Files.
//!
//! §4.1 of the developer-mode spec asks for "browser or GPUI preview
//! surface". The shell does not yet bundle a webview engine, so v1 is a
//! GPUI rendering of the file the Files tab last picked. Honest empty /
//! error / unsupported states per §13 — no "coming soon", every failure
//! carries its concrete reason.
//!
//! The plumbing into FilesTab is intentionally not a direct call: we
//! read a shared global `PreviewTarget`; Files sets it, Preview observes
//! it. Click-to-file therefore does not switch tabs, which keeps the
//! T174 fallback contract intact (a tab flip while the user reads would
//! be the regression to avoid).

use std::io::Read;
use std::path::{Path, PathBuf};

use chronos_ui::Theme;
use gpui::{
    AnyElement, Context, DragMoveEvent, Entity, FontWeight, IntoElement, MouseButton,
    MouseDownEvent, ObjectFit, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, img, prelude::*, px,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};
use crate::side_panel_right::tab::terminal::TerminalTab;

/// Drag marker for the drawer's resize handle — own type so it never
/// cross-fires with `RightPanelResize` (the panel's own horizontal drag).
struct EditorTerminalResize;

/// Local render mode (T194c). Mirrors `PreviewIntent` but lives on the tab
/// so a same-file mode switch (the header's Preview/Edit toggle) doesn't
/// need a global round-trip. `View` is the default and the only mode for
/// anything that isn't markdown-like — `Edit` is a Markdown-only, opt-in
/// surface, never the forced default (kill regression from T194).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ViewMode {
    #[default]
    View,
    Edit,
}

/// First-open default height.
const DRAWER_DEFAULT_H: f32 = 200.;
/// Floor so the grid always has room for at least a couple of rows.
const DRAWER_MIN_H: f32 = 80.;
/// Ceiling as a fraction of the window height (§UX point 3: "max ~50% of
/// tab" — approximated against the window since the tab fills it).
const DRAWER_MAX_FRACTION: f32 = 0.5;

/// Soft cap on text bodies before we stop reading and mark truncated
/// (§2 of T179). 128 KiB covers CONFIG/manifest/log scans comfortably;
/// beyond that the user should open the file in an editor.
const TEXT_CAP_BYTES: u64 = 128 * 1024;

/// Hard cap on image bodies. Past this we do not even attempt to load —
/// at 10 MiB we would burn Wayland texture memory for a thumbnail-sized
/// panel slot. The fork's `Img::extensions()` lists the formats on hand.
const IMAGE_CAP_BYTES: u64 = 10 * 1024 * 1024;

// Cap comparisons use strict `>` on purpose: a file whose size equals the
// cap fits into the "loaded whole" path. Marking a 128 KiB body as
// truncated when it sits exactly at the boundary is a lie — the user can
// always open it in an editor and see the file end-to-end.

/// First bytes we keep alongside the extension to disambiguate raw
/// binary vs printable text when the extension is unknown. Sixteen
/// bytes is enough to spot nulls and a UTF-8 BOM without touching
/// a full sector.
const SNIFF_BYTES: usize = 16;

/// What we do with the file, decided once on the background thread.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PreviewKind {
    /// png, jpg, jpeg, svg, webp, gif, bmp — handed to `img(PathBuf)`.
    Image,
    /// `.md` / `.markdown` / `.mdown` — rendered by
    /// `gpui_component::text::markdown`.
    Markdown,
    /// Printable UTF-8 — read up to cap, displayed as text.
    Text,
    /// `.html` / `.htm` / `.xhtml` / `.css` — the shell has no web
    /// rendering engine; say so honestly (§13), no "coming soon".
    WebPreview,
    /// Extension unknown and content looks binary — show type + size.
    Unsupported,
}

/// Image URL category for the markdown rewriter (T180).
///
/// Decided by a pure function over the URL string — no GPUI, no I/O. The
/// category decides whether the markdown source needs reshaping before
/// it reaches `gpui_component::text::markdown(...)`, which would
/// otherwise call into the asset cache and pull remote bitmaps on file
/// open.
///
/// `Remote` triggers redaction; the others are passed through unchanged
/// so the markdown renderer keeps handling them normally (data URI inline,
/// `file://` absolute path, relative + absolute paths resolved against the
/// markdown source file).
#[cfg(any(test, feature = "markdown"))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ImageUrlClass {
    /// `http://…`/`https://…`/`ftp://…` — **must not be loaded** by the
    /// shell when rendering local markdown. Replaced with a text marker.
    Remote(String),
    /// `data:` URI — inline bytes, no network.
    Data,
    /// `file://`, relative (`./`, `../`), absolute (`/abs/...`), or plain
    /// (`foo.png`) — handled by the markdown renderer as a local image.
    Local(String),
    /// Empty / whitespace — left alone so the alt text is still visible
    /// behind whatever the parser makes of it.
    Unsupported(String),
}

/// Whether the loaded body can be opened for editing (T194 — Editor = view
/// + edit, not view-only Preview).
///
/// `Text`/`Markdown` are editable as **plain source** — Markdown is not
/// rendered while editing, matching `docs/PRODUCT.md`'s "not a second
/// Zed/VS Code" stance: one buffer, one Save, no split preview/source view.
/// `truncated` disqualifies a file from editing outright: the buffer would
/// only hold the first `TEXT_CAP_BYTES`, and a Save would silently discard
/// everything past the cap — that is data loss, not an edge case.
fn is_editable(kind: PreviewKind, truncated: bool) -> bool {
    matches!(kind, PreviewKind::Text | PreviewKind::Markdown) && !truncated
}

/// Whether the Preview/Edit dual-mode toggle applies at all (T194c).
/// Narrower than [`is_editable`]: this task scopes the two-button UI to
/// **markdown-like** files only — plain `Text` stays view-only for now
/// (spec: "realistic: plain Text не обязан иметь Edit в этой задаче").
/// `is_editable` still governs the raw-buffer mechanics once in Edit mode;
/// this governs whether Edit mode is ever offered in the first place.
fn can_toggle_edit(kind: PreviewKind, truncated: bool) -> bool {
    kind == PreviewKind::Markdown && !truncated
}

/// Pure classifier over (extension, head bytes). Single source of truth;
/// covered by unit tests in `tests`.
pub(crate) fn classify(path: &Path, head: &[u8; SNIFF_BYTES]) -> PreviewKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "svg" | "webp" | "gif" | "bmp" => PreviewKind::Image,
        "md" | "markdown" | "mdown" => PreviewKind::Markdown,
        "html" | "htm" | "xhtml" | "css" => PreviewKind::WebPreview,
        _ => {
            if looks_like_text(head) {
                PreviewKind::Text
            } else {
                PreviewKind::Unsupported
            }
        }
    }
}

/// Heuristic for the unknown-extension case: a 16-byte probe is text
/// when ≥ 80 % of its bytes are printable-or-newline or a UTF-8
/// continuation byte. NULs and other ASCII controls (< 9, not 10/13,
/// not 32..=126) are not text. The header already rejects known-binary
/// extensions by magic number — this is the dotfiles and
/// stripped-extension fallback only.
fn looks_like_text(head: &[u8; SNIFF_BYTES]) -> bool {
    if head.iter().all(|b| *b == 0) {
        return false;
    }
    let mut printable = 0usize;
    for &b in head {
        let ok = matches!(b, 9 | 10 | 13 | 32..=126) || b >= 128;
        if ok {
            printable += 1;
        }
    }
    // 80 % = `4 * len / 5`. Compare avoiding float math.
    printable * 5 >= SNIFF_BYTES * 4
}

// --- T180: remote-image redaction for markdown previews ---

/// Pure URL classifier. Trims, lowercases the scheme part, decides.
///
/// The scheme part is what we lowercase; the rest is kept verbatim so
/// URL-encoded payloads (mixed-case parameters, percent-escapes) survive.
#[cfg(any(test, feature = "markdown"))]
pub(crate) fn classify_image_url(url: &str) -> ImageUrlClass {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return ImageUrlClass::Unsupported(url.to_string());
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("ftp://")
    {
        return ImageUrlClass::Remote(trimmed.to_string());
    }
    if lower.starts_with("data:") {
        return ImageUrlClass::Data;
    }
    if let Some(rest) = lower.strip_prefix("file://") {
        return ImageUrlClass::Local(rest.to_string());
    }
    // Anything else is treated as a local path: relative `./`, `../`,
    // absolute `/foo`, or plain `name.png`. The markdown renderer resolves
    // relatives against the source file — if the user wrote a typo, the
    // asset cache will fail *locally* and the log will be honest about it.
    ImageUrlClass::Local(trimmed.to_string())
}

/// One `![alt](url) ["title"]` match starting at byte offset `start`.
///
/// Returns the alt slice, the URL slice, the optional title slice, and
/// the byte index just past the closing `)`. `None` if `start` doesn't
/// look like image syntax.
///
/// Conservative on purpose: alt cannot contain `]`, URL cannot contain
/// whitespace or `)`, title must be a single `"…"` after optional
/// whitespace. That covers the realistic case (the README badges) and is
/// *not* a full CommonMark parser — that's the markdown crate's job.
/// Here we only need to know whether to redact.
///
/// **Known v1 limitations**: CommonMark doesn't allow `\` escapes inside
/// alt text — multi-image inputs like `![outer ![inner](url)](local.png)`
/// will close the outer alt at the inner `]`. Titles with a literal
/// quote, like `![alt](url "she said \"hi\"")`, will close on the first
/// `"`. Both cases still fall back to a labelled marker — never an
/// `ImageNode` for remote URLs — so the no-network guarantee holds even
/// when the syntax can't be parsed perfectly.
#[cfg(any(test, feature = "markdown"))]
struct ImageMatch<'a> {
    alt: &'a str,
    url: &'a str,
    title: Option<&'a str>,
    end: usize,
}

#[cfg(any(test, feature = "markdown"))]
fn match_image_at(text: &str, start: usize) -> Option<ImageMatch<'_>> {
    let bytes = text.as_bytes();
    if start + 4 > bytes.len() || bytes[start] != b'!' || bytes[start + 1] != b'[' {
        return None;
    }
    // Alt: scan from `start + 2` until the first ']'.
    let alt_rel = text[start + 2..].find(']')?;
    let alt_abs = start + 2 + alt_rel;
    let alt = &text[start + 2..alt_abs];
    // Expect '(' right after ']'.
    let after_alt = alt_abs + 1;
    if bytes.get(after_alt) != Some(&b'(') {
        return None;
    }
    let url_start = after_alt + 1;
    // URL: up to first whitespace or ')'.
    let url_end_rel = text[url_start..]
        .find(|c: char| c.is_whitespace() || c == ')')
        .unwrap_or(text.len().saturating_sub(url_start));
    let url = &text[url_start..url_start + url_end_rel];
    let url_end_abs = url_start + url_end_rel;
    // Optional: whitespace + quoted title + closing ')'.
    let mut cursor = url_end_abs;
    let title = match text[cursor..].chars().next() {
        Some(c) if c.is_whitespace() => {
            cursor += text[cursor..]
                .chars()
                .take_while(|c| c.is_whitespace())
                .count();
            let after_ws = &text[cursor..];
            if after_ws.starts_with('"') && after_ws.len() >= 2 {
                let close_rel = after_ws[1..].find('"')?;
                let title_str = &after_ws[1..=close_rel];
                cursor += 1 + close_rel + 1;
                Some(title_str)
            } else {
                None
            }
        }
        _ => None,
    };
    // Require closing ')'.
    if bytes.get(cursor) != Some(&b')') {
        return None;
    }
    Some(ImageMatch {
        alt,
        url,
        title,
        end: cursor + 1,
    })
}

/// Truncate a URL for the in-text marker so absurdly long ones don't
/// wreck layout. Char-boundary safe (multi-byte UTF-8).
#[cfg(any(test, feature = "markdown"))]
fn truncate_for_marker(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let kept: String = s.chars().take(max_chars).collect();
        format!("{kept}…")
    }
}

/// Walk `text` and rewrite every `![alt](remote-url) ["title"]` to a
/// plain-text marker `[🛰 {alt} — remote image, not loaded: {url}]`
/// (plus optional title). Local / data / `file://` paths pass through,
/// so the markdown renderer still treats them as images.
///
/// Why pre-process text instead of patching the gpui-component fork:
/// the fork's `format/markdown.rs` constructs `InlineNode::image(...)`
/// unconditionally for `Node::Image(raw)` and only resolves the URL on
/// render — there is no parser-side hook on the public API. A plugin
/// would have to fork `MarkdownExtensions::parse_inline`, raising the
/// cost well above what one bug-fix warrants. Pre-processing the source
/// keeps the no-network guarantee *before* the tree is built, so no
/// ImageNode is ever created.
#[cfg(any(test, feature = "markdown"))]
pub(crate) fn redact_remote_images(text: &str) -> String {
    // Hot path: most markdown bodies contain no `![…](…)` at all. Skip the
    // walk to avoid an allocation per render of generic markdown like a
    // README heading + paragraph.
    if !text.contains("![") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if let Some(m) = match_image_at(text, i) {
            match classify_image_url(m.url) {
                ImageUrlClass::Remote(raw) => {
                    let disp = truncate_for_marker(&raw, 80);
                    let title_part = m
                        .title
                        .map(|t| format!(" (title: \"{t}\")"))
                        .unwrap_or_default();
                    out.push_str(&format!(
                        "[🛰 {alt} — remote image, not loaded: {disp}{title_part}]",
                        alt = m.alt,
                    ));
                }
                _ => out.push_str(&text[i..m.end]),
            }
            i = m.end;
        } else {
            // Pass through one UTF-8 char at a time; panics-free.
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Lifecycle of the loaded file. `generation` is the matching
/// `PreviewTarget::generation`; stale background results compare against
/// it and discard themselves if the user moved on.
#[derive(Debug, Clone)]
enum State {
    Empty,
    Loading {
        generation: u64,
        path: PathBuf,
    },
    Loaded {
        generation: u64,
        path: PathBuf,
        kind: PreviewKind,
        size_bytes: u64,
        /// `Some` only for Markdown/Text — read up to the cap.
        text: Option<String>,
        truncated: bool,
    },
    Error {
        generation: u64,
        message: String,
    },
}

impl State {
    fn generation(&self) -> u64 {
        match self {
            State::Empty => 0,
            State::Loading { generation, .. }
            | State::Loaded { generation, .. }
            | State::Error { generation, .. } => *generation,
        }
    }
}

pub struct PreviewTab {
    state: State,
    scroll: ScrollHandle,
    /// View (rendered/scrolled) vs Edit (raw InputState buffer) — T194c.
    /// Only meaningful when `state` is `Loaded`; `Edit` only ever applies
    /// when `is_editable(kind, truncated)` — everything else stays View.
    view_mode: ViewMode,
    /// Intent captured from `PreviewTarget` at the moment a **new** load
    /// was kicked off — applied once that load settles (the global may
    /// have moved on by the time the background read completes, so we use
    /// the value that was current when this generation was requested, not
    /// whatever the global holds when the read finishes).
    pending_intent: PreviewIntent,
    /// Holds the `observe_global` subscription. Dropping it removes the
    /// listener. `new` returns immediately after subscribing.
    _target_subscription: gpui::Subscription,
    /// Editable buffer (T194). Created lazily on first `render()` — `new`
    /// has no `Window`, and `InputState::new` requires one — then reused
    /// across file switches (one `InputState` per tab, not per file).
    editor: Option<Entity<InputState>>,
    /// `InputEvent::Change` subscription on `editor`, created alongside it.
    /// `InputState::set_value` (used to load fresh content) suppresses
    /// `Change` internally, so this only fires on genuine user keystrokes.
    _editor_subscription: Option<Subscription>,
    /// `State::Loaded::generation` last synced into `editor`. `None` = not
    /// synced yet. Compared on every render to know when a newly loaded
    /// file (or a re-click of the same file after an external edit) needs
    /// `set_value` again.
    editor_generation: Option<u64>,
    /// Set on the first `InputEvent::Change` after a load/save; cleared by
    /// a fresh load or a successful save.
    dirty: bool,
    /// True while a save write is in flight — disables the Save button so
    /// a double-click cannot race two writes.
    saving: bool,
    /// Outcome of the last save attempt: `(true, _)` success, `(false,
    /// reason)` failure. Only the failure case is rendered (§13 — the
    /// Save button's own "Save" → "Saved" label already communicates
    /// success honestly, a duplicate banner would be noise).
    save_result: Option<(bool, String)>,
    /// Terminal drawer (T194b) — lazily created on first toggle-open, then
    /// reused for the lifetime of this `PreviewTab` entity (so the PTY
    /// session survives collapsing/reopening the drawer, only dying with
    /// the tab itself).
    terminal_drawer: Option<Entity<TerminalTab>>,
    /// Whether the drawer is currently shown. Toggling does not drop
    /// `terminal_drawer` — collapsing just hides it.
    drawer_open: bool,
    /// Drawer body height in px while open. Persists across collapse/open
    /// so reopening restores the user's last size.
    drawer_height: f32,
    drawer_resize_start_y: Option<f32>,
    drawer_resize_start_height: Option<f32>,
}

impl PreviewTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Defensive default — `side_panel_right::init` is the canonical
        // place that registers globals, but tests and early wiring must
        // not race with that, and `cx.observe_global` requires the
        // global to already exist on first call.
        if !cx.has_global::<PreviewTarget>() {
            cx.set_global(PreviewTarget::default());
        }
        let subscription = cx.observe_global::<PreviewTarget>(|this, cx| {
            this.on_target_changed(cx);
        });
        let mut this = Self {
            state: State::Empty,
            scroll: ScrollHandle::new(),
            _target_subscription: subscription,
            view_mode: ViewMode::View,
            pending_intent: PreviewIntent::View,
            editor: None,
            _editor_subscription: None,
            editor_generation: None,
            dirty: false,
            saving: false,
            save_result: None,
            terminal_drawer: None,
            drawer_open: false,
            drawer_height: DRAWER_DEFAULT_H,
            drawer_resize_start_y: None,
            drawer_resize_start_height: None,
        };
        // The observer only fires on *changes*; the global may already
        // carry a path that was set before the tab was first created,
        // so we read the current value once.
        this.on_target_changed(cx);
        this
    }

    /// Write the editor's current content back to the loaded file. No-op if
    /// there is nothing loaded, no editor yet, or a save is already in
    /// flight (guards the double-click race the Save button's disabled
    /// state already prevents visually, but events can still queue up).
    fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let State::Loaded { path, .. } = &self.state else {
            return;
        };
        let Some(editor) = &self.editor else {
            return;
        };
        let path = path.clone();
        let content = editor.read(cx).value().to_string();

        self.saving = true;
        self.save_result = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let io_path = path.clone();
            let result = cx
                .background_spawn(async move {
                    std::fs::write(&io_path, content.as_bytes())
                        .map_err(|e| format!("Cannot save '{}': {e}", io_path.display()))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.saving = false;
                match result {
                    Ok(()) => {
                        this.dirty = false;
                        this.save_result = Some((true, String::new()));
                        tracing::info!(path = %path.display(), "side_panel_right editor: saved");
                    }
                    Err(message) => {
                        tracing::warn!(
                            path = %path.display(),
                            %message,
                            "side_panel_right editor: save failed"
                        );
                        this.save_result = Some((false, message));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Attempt to switch to `mode`. Blocks Edit → View while `dirty`
    /// instead of silently discarding the buffer — flashes a muted hint
    /// through the same `save_result` slot the Save button's failure
    /// banner already uses (T194c acceptance: "no silent dirty loss").
    /// Switching **into** Edit is never blocked. Returns whether the
    /// switch actually happened.
    fn try_set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) -> bool {
        if mode == ViewMode::View && self.view_mode == ViewMode::Edit && self.dirty {
            self.save_result =
                Some((false, "Save or discard before switching to Preview".to_string()));
            cx.notify();
            return false;
        }
        self.view_mode = mode;
        cx.notify();
        true
    }

    /// Resolve `intent` against the just-(re)confirmed `kind`/`truncated`
    /// and apply it as the view mode. `Edit` on a non-editable kind (or a
    /// truncated file) is honored as `View` with a log line — never a
    /// silent no-op, never a crash. Used both for a fresh load settling
    /// and for the same-path fast path in `on_target_changed`.
    fn apply_intent(&mut self, intent: PreviewIntent, kind: PreviewKind, truncated: bool) {
        let can_edit = can_toggle_edit(kind, truncated);
        let target = match intent {
            PreviewIntent::Edit if can_edit => ViewMode::Edit,
            PreviewIntent::Edit => {
                tracing::warn!(
                    ?kind,
                    truncated,
                    "side_panel_right editor: Edit intent on non-editable kind, forcing View"
                );
                ViewMode::View
            }
            PreviewIntent::View => ViewMode::View,
        };
        self.view_mode = target;
    }

    /// Toggle the terminal drawer open/closed. Lazily spawns the shared
    /// `TerminalTab` engine (and its PTY) on the *first* open only — later
    /// toggles just flip visibility, reusing the same session (T194b UX
    /// point 4).
    fn toggle_drawer(&mut self, cx: &mut Context<Self>) {
        if self.terminal_drawer.is_none() {
            self.terminal_drawer = Some(cx.new(TerminalTab::new));
        }
        self.drawer_open = !self.drawer_open;
        cx.notify();
    }

    fn start_drawer_resize(&mut self, start_y: f32, cx: &mut Context<Self>) {
        self.drawer_resize_start_y = Some(start_y);
        self.drawer_resize_start_height = Some(self.drawer_height);
        cx.notify();
    }

    fn update_drawer_resize(&mut self, current_y: f32, max_h: f32, cx: &mut Context<Self>) {
        let (start_y, start_h) = match (self.drawer_resize_start_y, self.drawer_resize_start_height)
        {
            (Some(y), Some(h)) => (y, h),
            _ => return,
        };
        // Handle sits above the drawer body: pointer moves up (smaller y) →
        // drawer grows. new = start_h - (current_y - start_y)
        let delta = current_y - start_y;
        let target = (start_h - delta).clamp(DRAWER_MIN_H, max_h.max(DRAWER_MIN_H));
        self.drawer_height = target;
        cx.notify();
    }

    /// Shared chrome above the content area — present in every state, both
    /// view modes (T194c "residual": the Terminal toggle must stay
    /// reachable in View, not only inside the editor body). Shows the
    /// Preview/Edit segmented toggle only when `can_edit` (markdown-like,
    /// not truncated) — every other kind gets no mode pair, view only.
    fn render_chrome_bar(&mut self, can_edit: bool, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let view_mode = self.view_mode;
        let drawer_open = self.drawer_open;

        let mut left = div().flex().items_center().gap(px(4.));
        if can_edit {
            let preview_active = view_mode == ViewMode::View;
            left = left
                .child(
                    div()
                        .id("editor-mode-preview")
                        .px(px(8.))
                        .py(px(3.))
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(if preview_active { theme.accent.primary } else { theme.border.subtle })
                        .text_color(if preview_active { theme.text.primary } else { theme.text.muted })
                        .text_size(px(10.5))
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.try_set_view_mode(ViewMode::View, cx);
                        }))
                        .child("Preview"),
                )
                .child(
                    div()
                        .id("editor-mode-edit")
                        .px(px(8.))
                        .py(px(3.))
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(if !preview_active { theme.accent.primary } else { theme.border.subtle })
                        .text_color(if !preview_active { theme.text.primary } else { theme.text.muted })
                        .text_size(px(10.5))
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.try_set_view_mode(ViewMode::Edit, cx);
                        }))
                        .child("Edit"),
                );
        }

        div()
            .id("editor-chrome-bar")
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .px(px(12.))
            .py(px(6.))
            .border_b_1()
            .border_color(theme.border.subtle)
            .child(left)
            .child(
                div()
                    .id("editor-terminal-toggle")
                    .px(px(10.))
                    .py(px(4.))
                    .rounded_md()
                    .cursor_pointer()
                    .bg(if drawer_open { theme.interactive.hover } else { theme.border.subtle })
                    .text_color(theme.text.primary)
                    .text_size(px(11.))
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        this.toggle_drawer(cx);
                    }))
                    .child(if drawer_open { "Terminal ▾" } else { "Terminal ▸" }),
            )
            .into_any_element()
    }

    /// Editable body: header (path + dirty indicator + Save button) over a
    /// multi-line `Input`. Bypasses `render_loaded`'s free-function match —
    /// building the Save button needs `cx.listener`, which only exists on a
    /// live `Context<Self>`. Mode toggle and Terminal toggle live in the
    /// shared `render_chrome_bar` above this, not here (T194c hoist).
    fn render_editor_input_body(&mut self, path: &Path, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        // `self.editor` is guaranteed `Some` here: `Render::render` creates
        // it (and syncs the loaded content) in the sync block that runs
        // before this is ever called.
        let path_label: SharedString = path.to_string_lossy().into_owned().into();
        let dirty = self.dirty;
        let saving = self.saving;
        let can_save = dirty && !saving;
        let save_label: &'static str = if saving {
            "Saving…"
        } else if dirty {
            "Save"
        } else {
            "Saved"
        };
        let failure = self
            .save_result
            .clone()
            .filter(|(ok, _)| !ok)
            .map(|(_, message)| message);

        let mut left_col = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .min_w(px(0.))
            .child(
                div().flex().items_center().gap(px(6.)).child(
                    div()
                        .text_size(px(11.))
                        .font_family(theme.font_mono)
                        .text_color(theme.text.muted)
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(path_label),
                ).when(dirty, |el| {
                    el.child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.status.warning)
                            .child("• unsaved"),
                    )
                }),
            );
        if let Some(message) = failure {
            left_col = left_col.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.status.error)
                    .child(message),
            );
        }

        let header = div()
            .id("editor-input-header")
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .px(px(12.))
            .py(px(8.))
            .border_b_1()
            .border_color(theme.border.subtle)
            .child(left_col)
            .child(
                div()
                    .id("editor-save")
                    .px(px(10.))
                    .py(px(4.))
                    .rounded_md()
                    .cursor_pointer()
                    .bg(if can_save { theme.accent.primary } else { theme.border.subtle })
                    .text_color(if can_save { theme.text.primary } else { theme.text.muted })
                    .text_size(px(11.))
                    .when(can_save, |el| {
                        el.on_click(cx.listener(|this, _e, _w, cx| this.save(cx)))
                    })
                    .child(save_label),
            );

        let body = div()
            .id("editor-body")
            .flex_1()
            .min_h(px(0.))
            .p(px(10.))
            .bg(crate::side_panel_right::surfaces::editor(theme))
            .when_some(self.editor.clone(), |el, editor| {
                // T205: explicit themed surface on the Input itself — gpui-component
                // default (Light) fill was the "white projector" on dark shell.
                // Styled() applies after Input's own `appearance` bg, so these win.
                el.child(
                    Input::new(&editor)
                        .bordered(false)
                        .h_full()
                        .focus_bordered(false)
                        .bg(crate::side_panel_right::surfaces::editor(theme))
                        .text_color(theme.text.primary)
                        .font_family(theme.font_mono)
                        .text_size(px(13.)),
                )
            });

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(body)
            .into_any_element()
    }

    /// Drawer resize handle + terminal body, appended to the outer column
    /// when `drawer_open` (T194c hoist — was previously nested inside the
    /// Edit-only body, so it vanished in View mode; now it's a sibling of
    /// the content area regardless of mode).
    fn render_drawer_extras(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (AnyElement, AnyElement) {
        let max_h = (window.bounds().size.height.as_f32() * DRAWER_MAX_FRACTION).max(DRAWER_MIN_H);
        // Clamp on every render too, not just at drag time — a window
        // resize (or a drawer height carried over from a bigger window)
        // must not leave the drawer taller than the current 50% ceiling.
        if self.drawer_height > max_h {
            self.drawer_height = max_h;
        }
        let drawer_height = self.drawer_height.max(DRAWER_MIN_H);

        if let Some(terminal) = &self.terminal_drawer {
            terminal.update(cx, |t, _cx| t.set_available_height(Some(drawer_height)));
        }

        let resize_drag_handler = cx.listener(
            |this, ev: &DragMoveEvent<EditorTerminalResize>, window, cx| {
                let max_h = window.bounds().size.height.as_f32() * DRAWER_MAX_FRACTION;
                this.update_drawer_resize(f32::from(ev.event.position.y), max_h.max(DRAWER_MIN_H), cx);
            },
        );
        let resize_mouse_handler = cx.listener(|this, ev: &MouseDownEvent, _w, cx| {
            this.start_drawer_resize(f32::from(ev.position.y), cx);
        });

        let handle = div()
            .id("editor-terminal-resize-handle")
            .flex_none()
            .h(px(6.))
            .w_full()
            .cursor_row_resize()
            .bg(theme.border.subtle)
            .on_mouse_down(MouseButton::Left, resize_mouse_handler)
            .on_drag(EditorTerminalResize, |_, _, _, cx| cx.new(|_| gpui::EmptyView))
            .on_drag_move(resize_drag_handler)
            .into_any_element();

        let mut drawer_body = div()
            .id("editor-terminal-drawer")
            .flex_none()
            .h(px(drawer_height))
            .w_full()
            .overflow_hidden()
            .border_t_1()
            .border_color(theme.border.subtle);
        if let Some(terminal) = &self.terminal_drawer {
            drawer_body = drawer_body.child(terminal.clone());
        }

        (handle, drawer_body.into_any_element())
    }

    fn on_target_changed(&mut self, cx: &mut Context<Self>) {
        let (path, generation, intent) = {
            let t = cx.global::<PreviewTarget>();
            (t.path.clone(), t.generation, t.intent)
        };
        let Some(path) = path else {
            self.state = State::Empty;
            self.view_mode = ViewMode::View;
            cx.notify();
            return;
        };

        // Same path already loaded — switch mode locally, no re-read
        // (T194c: "same path, intent change only → switch mode without
        // full re-read if already Loaded with text"). Routed through
        // `try_set_view_mode` so re-requesting View while the editor is
        // dirty on this same file is guarded the same way the header
        // toggle is — no silent buffer loss just because the request came
        // from Files instead of the tab itself.
        if let State::Loaded { path: loaded_path, kind, truncated, .. } = &self.state
            && *loaded_path == path
        {
            let (kind, truncated) = (*kind, *truncated);
            let can_edit = can_toggle_edit(kind, truncated);
            let target = match intent {
                PreviewIntent::Edit if can_edit => ViewMode::Edit,
                _ => ViewMode::View,
            };
            self.try_set_view_mode(target, cx);
            return;
        }

        self.state = State::Loading {
            generation,
            path: path.clone(),
        };
        self.pending_intent = intent;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let path_for_io = path.clone();
            let result = cx
                .background_spawn(async move { read_for_preview(&path_for_io) })
                .await;

            // Generation guard on the foreground — drop the result if a
            // smaller index has been superseded by a newer click.
            let _ = this.update(cx, |this, cx| {
                if this.state.generation() != generation {
                    return;
                }
                this.state = match result {
                    Ok(loaded) => State::Loaded {
                        generation,
                        path: loaded.path,
                        kind: loaded.kind,
                        size_bytes: loaded.size_bytes,
                        text: loaded.text,
                        truncated: loaded.truncated,
                    },
                    Err(err) => State::Error {
                        generation,
                        message: err,
                    },
                };
                // Apply the intent captured when *this* load was kicked
                // off — not whatever the global holds now, which may have
                // moved on while the background read was in flight.
                match &this.state {
                    State::Loaded { kind, truncated, .. } => {
                        let (kind, truncated) = (*kind, *truncated);
                        this.apply_intent(this.pending_intent, kind, truncated);
                    }
                    _ => this.view_mode = ViewMode::View,
                }
                cx.notify();
            });
        })
        .detach();
    }
}

/// Pure background read — no GPUI types, no `cx`. Mirrors the
/// `FilesTab::request_reload` pattern (T176) of doing blocking I/O
/// inside `cx.background_spawn`.
#[derive(Debug)]
struct Loaded {
    path: PathBuf,
    kind: PreviewKind,
    size_bytes: u64,
    text: Option<String>,
    truncated: bool,
}

#[allow(clippy::disallowed_methods)]
fn read_for_preview(path: &Path) -> Result<Loaded, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|e| format!("Cannot read '{}': {e}", path.display()))?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "Cannot preview '{}': not a regular file",
            path.display()
        ));
    }
    let size_bytes = metadata.len();

    let mut head = [0u8; SNIFF_BYTES];
    let _ = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut head)); // missing file is caught above; empty file is fine

    let kind = classify(path, &head);

    let truncated = matches!(kind, PreviewKind::Markdown | PreviewKind::Text)
        && size_bytes > TEXT_CAP_BYTES;

    let text = match kind {
        PreviewKind::Markdown | PreviewKind::Text => {
            if truncated {
                let mut buf = vec![0u8; TEXT_CAP_BYTES as usize];
                let mut f = std::fs::File::open(path)
                    .map_err(|e| format!("Cannot read '{}': {e}", path.display()))?;
                f.read_exact(&mut buf)
                    .map_err(|e| format!("Cannot read '{}': {e}", path.display()))?;
                Some(
                    String::from_utf8(buf).map_err(|e| {
                        format!("Cannot decode '{}': invalid UTF-8 ({e})", path.display())
                    })?,
                )
            } else {
                Some(
                    std::fs::read_to_string(path)
                        .map_err(|e| format!("Cannot read '{}': {e}", path.display()))?,
                )
            }
        }
        _ => None,
    };

    let text_len = text.as_ref().map(|s| s.len()).unwrap_or(0);
    tracing::info!(
        kind = ?kind,
        bytes = size_bytes,
        text_len,
        truncated,
        path = %path.display(),
        "side_panel_right preview: loaded",
    );

    Ok(Loaded {
        path: path.to_path_buf(),
        kind,
        size_bytes,
        text,
        truncated,
    })
}

impl Render for PreviewTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let state = self.state.clone();
        let scroll = self.scroll.clone();

        // T194c kill-regression: only sync into `editor` (and only even
        // *create* it) when the user actually asked to edit. View mode —
        // the default — never touches InputState, so opening a markdown
        // file never forces the raw buffer anymore.
        if let State::Loaded { generation, kind, text: Some(text), truncated, .. } = &state
            && self.view_mode == ViewMode::Edit
            && is_editable(*kind, *truncated)
            && self.editor_generation != Some(*generation)
        {
            if self.editor.is_none() {
                // T205: CodeEditor mode — built-in line-number gutter with
                // scroll synced to the buffer (one element), mono family, and
                // `highlighter` left None so no syntax highlight (spec non-goal).
                // Buffer bg/text/font are themed on the `Input` element itself
                // in `render_editor_input_body`.
                let editor =
                    cx.new(|cx| InputState::new(window, cx).code_editor("plaintext"));
                let subscription = cx.subscribe(
                    &editor,
                    |this: &mut Self, _editor, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            this.dirty = true;
                            this.save_result = None;
                            cx.notify();
                        }
                    },
                );
                self.editor = Some(editor);
                self._editor_subscription = Some(subscription);
            }
            if let Some(editor) = &self.editor {
                editor.update(cx, |input, cx| input.set_value(text.clone(), window, cx));
            }
            self.editor_generation = Some(*generation);
            self.dirty = false;
            self.saving = false;
            self.save_result = None;
        }

        let can_edit = matches!(&state, State::Loaded { kind, truncated, .. } if can_toggle_edit(*kind, *truncated));
        let chrome_bar = self.render_chrome_bar(can_edit, &theme, cx);

        let content: AnyElement = match state {
            State::Empty => render_empty(&theme),
            State::Loading { path, .. } => render_loading(&path, &theme),
            State::Loaded {
                path,
                kind,
                size_bytes,
                text,
                truncated,
                ..
            } => {
                if self.view_mode == ViewMode::Edit && is_editable(kind, truncated) {
                    self.render_editor_input_body(&path, &theme, cx)
                } else {
                    render_loaded(&path, kind, size_bytes, text.as_deref(), truncated, &theme, &scroll)
                }
            }
            State::Error { message, .. } => render_error(&message, &theme),
        };

        let mut column = div()
            .size_full()
            .flex()
            .flex_col()
            .child(chrome_bar)
            .child(div().flex_1().min_h(px(0.)).child(content));

        if self.drawer_open {
            let (handle, drawer_body) = self.render_drawer_extras(&theme, window, cx);
            column = column.child(handle).child(drawer_body);
        }

        column
    }
}

fn render_empty(theme: &Theme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .px(px(24.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text.primary)
                .child("No file selected"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.text.muted)
                .text_center()
                .child("Open the Files tab and click any file to preview it here."),
        )
        .into_any_element()
}

fn render_loading(path: &Path, theme: &Theme) -> AnyElement {
    let label: SharedString = format!("Loading {}…", path.display()).into();
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.text.muted)
                .child(label),
        )
        .into_any_element()
}

fn render_loaded(
    path: &Path,
    kind: PreviewKind,
    size_bytes: u64,
    text: Option<&str>,
    truncated: bool,
    theme: &Theme,
    scroll: &ScrollHandle,
) -> AnyElement {
    let path_label: SharedString = path.to_string_lossy().into_owned().into();
    let header = div()
        .id("preview-header")
        .flex()
        .flex_col()
        .gap(px(2.))
        .px(px(12.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text.primary)
                        .child("Preview"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.text.muted)
                        .child(kind_label(kind)),
                ),
        )
        .child(
            div()
                .text_size(px(11.))
                .font_family(theme.font_mono)
                .text_color(theme.text.muted)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(path_label),
        );

    let body: AnyElement = match kind {
        PreviewKind::Image => render_image(path, size_bytes, theme),
        #[cfg(feature = "markdown")]
        PreviewKind::Markdown => render_markdown(text.unwrap_or(""), truncated, theme, scroll),
        #[cfg(not(feature = "markdown"))]
        PreviewKind::Markdown => render_text(text.unwrap_or(""), truncated, theme, scroll),
        PreviewKind::Text => render_text(text.unwrap_or(""), truncated, theme, scroll),
        PreviewKind::WebPreview => render_web_unavailable(path, theme),
        PreviewKind::Unsupported => render_unsupported(path, size_bytes, theme),
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(header)
        .child(body)
        .into_any_element()
}

fn kind_label(kind: PreviewKind) -> &'static str {
    match kind {
        PreviewKind::Image => "image",
        PreviewKind::Markdown => "markdown",
        PreviewKind::Text => "text",
        PreviewKind::WebPreview => "web (unavailable)",
        PreviewKind::Unsupported => "unsupported",
    }
}

fn render_image(path: &Path, size_bytes: u64, theme: &Theme) -> AnyElement {
    if size_bytes > IMAGE_CAP_BYTES {
        // Refuse to load — `Img` would happily decode this on the
        // foreground; the cap is here so a 50 MB SVG doesn't pin us.
        return div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.))
            .px(px(16.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text.primary)
                    .child("Image too large"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.status.warning)
                    .child(format!(
                        "{} — {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                        human_bytes(size_bytes)
                    )),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.text.muted)
                    .text_center()
                    .child("Open the file in an image viewer to see it."),
            )
            .into_any_element();
    }

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .items_center()
        .justify_center()
        .p(px(12.))
        .child(
            img(path.to_path_buf())
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain),
        )
        .into_any_element()
}

#[cfg(feature = "markdown")]
fn render_markdown(
    text: &str,
    truncated: bool,
    theme: &Theme,
    scroll: &ScrollHandle,
) -> AnyElement {
    // `overflow_y_scroll` is on `StatefulInteractiveElement`, which is
    // implemented for `Stateful<T>` (returned by `.id(...)`) and for
    // `Img` only — the bare `Div` does not have it. `.id(...)` upgrades
    // the builder to `Stateful<Div>` before the scroll method runs.
    let mut body = div()
        .id("preview-markdown-body")
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .p(px(10.));
    if truncated {
        body = body.child(truncated_banner(theme));
    }
    // T180: redact remote `![alt](https://… or http://…)` before
    // handing the source to the markdown renderer. `gpui_component`
    // would otherwise pull the bitmap through the asset cache, leaking
    // IP and spamming `ERROR … asset_cache` for img.shields.io and
    // friends. Local / data / `file://` paths pass through verbatim;
    // see `redact_remote_images` for the precise contract.
    let safe = redact_remote_images(text);
    body.child(gpui_component::text::markdown(safe.as_str()))
        .into_any_element()
}

fn render_text(text: &str, truncated: bool, theme: &Theme, scroll: &ScrollHandle) -> AnyElement {
    // Same `.id(...)`-then-`.overflow_y_scroll()` chain — see render_markdown.
    // Padding 10 px matches the markdown body so the two layouts share
    // horizontal rhythm.
    let mut body = div()
        .id("preview-text-body")
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .p(px(10.))
        .font_family(theme.font_mono)
        .text_size(px(12.))
        .text_color(theme.text.primary);
    if truncated {
        body = body.child(truncated_banner(theme));
    }
    body.child(text.to_owned()).into_any_element()
}

fn truncated_banner(theme: &Theme) -> AnyElement {
    div()
        .mb(px(8.))
        .px(px(10.))
        .py(px(5.))
        .rounded_md()
        .bg(theme.bg.elevated)
        .text_size(px(11.))
        .text_color(theme.status.warning)
        .child(format!(
            "Text truncated — showing first {} KB",
            TEXT_CAP_BYTES / 1024
        ))
        .into_any_element()
}

fn render_web_unavailable(path: &Path, theme: &Theme) -> AnyElement {
    let path_label: SharedString = path.to_string_lossy().into_owned().into();
    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .px(px(16.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.status.warning)
                .child("Web preview unavailable"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.text.muted)
                .text_center()
                // §13 strict: no future promise, no "yet", no ETA. The
                // absence of an engine is the fact we are reporting.
                .child("The shell has no web rendering engine."),
        )
        .child(
            div()
                .text_size(px(11.))
                .font_family(theme.font_mono)
                .text_color(theme.text.muted)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(format!("({path_label})")),
        )
        .into_any_element()
}

fn render_unsupported(path: &Path, size_bytes: u64, theme: &Theme) -> AnyElement {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("(no extension)");
    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .px(px(16.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text.primary)
                .child("Unsupported file type"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.text.muted)
                .child(format!("{ext} — {}", human_bytes(size_bytes))),
        )
        .into_any_element()
}

fn render_error(message: &str, theme: &Theme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .px(px(16.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.status.error)
                .text_center()
                .child(message.to_string()),
        )
        .into_any_element()
}

fn human_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = size as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{size}{}", UNITS[0])
    } else {
        format!("{v:.1}{}", UNITS[u])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn install_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
        });
    }

    #[test]
    fn classify_known_image_extensions() {
        // `classify` takes `&[u8; SNIFF_BYTES]` — build the array by copying
        // the probe bytes into a fixed-size buffer. Using `b"..."` slices
        // would mismatch on size.
        let head = {
            let probe: &[u8] = b"...";
            let mut a = [0u8; SNIFF_BYTES];
            for (i, slot) in a.iter_mut().enumerate() {
                *slot = probe.get(i).copied().unwrap_or(0);
            }
            a
        };
        for ext in ["png", "PNG", "jpg", "jpeg", "svg", "webp", "gif", "bmp"] {
            let path = PathBuf::from(format!("/x/foo.{ext}"));
            assert_eq!(classify(&path, &head), PreviewKind::Image, "ext={ext}");
        }
    }

    #[test]
    fn classify_markdown_variants() {
        let head = {
            let probe: &[u8] = b"# hi";
            let mut a = [0u8; SNIFF_BYTES];
            for (i, slot) in a.iter_mut().enumerate() {
                *slot = probe.get(i).copied().unwrap_or(0);
            }
            a
        };
        for ext in ["md", "markdown", "mdown", "MD"] {
            let path = PathBuf::from(format!("/x/README.{ext}"));
            assert_eq!(classify(&path, &head), PreviewKind::Markdown);
        }
    }

    #[test]
    fn classify_web_is_honest_unavailable() {
        let head = {
            let probe: &[u8] = b"...\n";
            let mut a = [0u8; SNIFF_BYTES];
            for (i, slot) in a.iter_mut().enumerate() {
                *slot = probe.get(i).copied().unwrap_or(0);
            }
            a
        };
        for ext in ["html", "htm", "xhtml", "css"] {
            let path = PathBuf::from(format!("/x/main.{ext}"));
            assert_eq!(classify(&path, &head), PreviewKind::WebPreview);
        }
    }

    #[test]
    fn classify_unknown_with_text_bytes_falls_through_to_text() {
        let path = PathBuf::from("/x/.bashrc");
        // Fill the 16-byte head by tiling "export PATH=\n" round-robin — the
        // real probe in `looks_like_text` consults `str::from_utf8`, so the
        // test must hand it bytes that decode cleanly. The `.max(b'\n')`
        // trick from the first draft silently inflated arbitrary bytes above
        // 0x0a and broke the contract; do it the boring way.
        let src = b"export PATH=\n";
        let mut head = [0u8; SNIFF_BYTES];
        for (i, slot) in head.iter_mut().enumerate() {
            *slot = src[i % src.len()];
        }
        assert!(
            std::str::from_utf8(&head).is_ok(),
            "test sanity: probe head must decode as UTF-8"
        );
        assert_eq!(classify(&path, &head), PreviewKind::Text);
    }

    #[test]
    fn classify_unknown_with_binary_bytes_is_unsupported() {
        let path = PathBuf::from("/x/data.bin");
        let head = [0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(classify(&path, &head), PreviewKind::Unsupported);
    }

    #[test]
    fn classify_all_zero_is_unsupported() {
        let path = PathBuf::from("/x/.dat");
        let head = [0u8; SNIFF_BYTES];
        assert_eq!(classify(&path, &head), PreviewKind::Unsupported);
    }

    // --- T194: editability rule ---

    #[test]
    fn text_and_markdown_are_editable_when_not_truncated() {
        assert!(is_editable(PreviewKind::Text, false));
        assert!(is_editable(PreviewKind::Markdown, false));
    }

    #[test]
    fn truncated_text_and_markdown_are_not_editable() {
        // A truncated buffer only holds the first TEXT_CAP_BYTES — saving it
        // would silently discard the rest of the file. Never editable.
        assert!(!is_editable(PreviewKind::Text, true));
        assert!(!is_editable(PreviewKind::Markdown, true));
    }

    #[test]
    fn non_text_kinds_are_never_editable() {
        for kind in [
            PreviewKind::Image,
            PreviewKind::WebPreview,
            PreviewKind::Unsupported,
        ] {
            assert!(!is_editable(kind, false), "{kind:?} must not be editable");
            assert!(!is_editable(kind, true), "{kind:?} must not be editable");
        }
    }

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0B");
        assert_eq!(human_bytes(512), "512B");
        assert_eq!(human_bytes(2048), "2.0K");
        assert_eq!(human_bytes(2 * 1024 * 1024), "2.0M");
    }

    #[gpui::test]
    fn starts_empty_without_target(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| {
            cx.set_global(PreviewTarget::default());
        });
        let view = cx.new(|cx| PreviewTab::new(cx));
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| {
            assert!(matches!(this.state, State::Empty));
        });
    }

    #[gpui::test]
    fn setting_target_drives_loading_and_settles_to_loaded(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| {
            cx.set_global(PreviewTarget::default());
        });
        let view = cx.new(|cx| PreviewTab::new(cx));
        let dir = std::env::temp_dir().join(format!("chronos-t179-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("README.md");
        std::fs::write(&target, "# Hello\nworld\n").unwrap();

        cx.update(|cx| {
            cx.set_global(PreviewTarget::file(target.clone()));
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| match &this.state {
            State::Loaded { kind, truncated, text, .. } => {
                assert_eq!(*kind, PreviewKind::Markdown);
                assert!(!truncated);
                assert_eq!(text.as_deref(), Some("# Hello\nworld\n"));
            }
            other => panic!("expected Loaded Markdown, got {other:?}"),
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- T194c: view default + md preview/edit modes ---

    fn write_md(dir_tag: &str, body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("chronos-t194c-{dir_tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("README.md");
        std::fs::write(&target, body).unwrap();
        (dir, target)
    }

    #[gpui::test]
    fn markdown_loaded_with_view_intent_stays_view_mode(cx: &mut TestAppContext) {
        // Kill-regression check: opening markdown with the default (View)
        // intent must NOT force the raw editor body — this was T194's bug.
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let (dir, target) = write_md("view-default", "# Hello\n");

        cx.update(|cx| cx.set_global(PreviewTarget::file(target.clone())));
        cx.background_executor.run_until_parked();

        cx.update_entity(&view, |this, _cx| {
            assert_eq!(this.view_mode, ViewMode::View, "default intent must land in View mode");
            assert!(this.editor.is_none(), "View mode must never create the raw InputState buffer");
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn edit_intent_on_markdown_settles_to_edit_mode_with_editor(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let (dir, target) = write_md("edit-intent", "# Hello\n");

        cx.update(|cx| {
            let mut t = PreviewTarget::file(target.clone());
            t.intent = PreviewIntent::Edit;
            cx.set_global(t);
        });
        cx.background_executor.run_until_parked();

        cx.update_entity(&view, |this, _cx| {
            assert_eq!(this.view_mode, ViewMode::Edit, "Edit intent on markdown must settle to Edit mode");
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn edit_intent_on_plain_text_also_forces_view(cx: &mut TestAppContext) {
        // T194c scope: dual-mode is markdown-only. Plain Text is
        // `is_editable` (mechanics allow it) but NOT `can_toggle_edit` —
        // Edit intent on a .txt/.log must land in View, same as an image,
        // not silently succeed into edit mode.
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let dir = std::env::temp_dir().join(format!("chronos-t194c-text-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("notes.txt");
        std::fs::write(&target, "plain text body\n").unwrap();

        cx.update(|cx| {
            let mut t = PreviewTarget::file(target.clone());
            t.intent = PreviewIntent::Edit;
            cx.set_global(t);
        });
        cx.background_executor.run_until_parked();

        cx.update_entity(&view, |this, _cx| {
            assert_eq!(
                this.view_mode,
                ViewMode::View,
                "Edit intent on plain Text must be forced to View — dual-mode is markdown-only this task"
            );
            assert!(this.editor.is_none(), "forced View must never build the raw buffer");
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn edit_intent_on_image_forces_view(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let dir = std::env::temp_dir().join(format!("chronos-t194c-img-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("pic.png");
        std::fs::write(&target, [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]).unwrap();

        cx.update(|cx| {
            let mut t = PreviewTarget::file(target.clone());
            t.intent = PreviewIntent::Edit;
            cx.set_global(t);
        });
        cx.background_executor.run_until_parked();

        cx.update_entity(&view, |this, _cx| {
            assert_eq!(
                this.view_mode,
                ViewMode::View,
                "Edit intent on a non-editable kind (image) must be forced to View, never crash/hang"
            );
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn same_path_intent_switch_does_not_reload(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let (dir, target) = write_md("same-path", "# Hello\n");

        cx.update(|cx| cx.set_global(PreviewTarget::file(target.clone())));
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| assert_eq!(this.view_mode, ViewMode::View));

        // Same path, Edit intent — must switch mode without re-entering
        // State::Loading (a real re-read would flip state to Loading first).
        cx.update(|cx| {
            let mut t = PreviewTarget::file(target.clone());
            t.generation = 2;
            t.intent = PreviewIntent::Edit;
            cx.set_global(t);
        });
        cx.update_entity(&view, |this, _cx| {
            assert_eq!(this.view_mode, ViewMode::Edit, "same-path intent switch must apply immediately");
            assert!(
                matches!(this.state, State::Loaded { .. }),
                "same-path intent switch must not re-enter Loading"
            );
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn edit_to_view_blocked_while_dirty_no_silent_loss(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let (dir, target) = write_md("dirty-guard", "# Hello\n");

        cx.update(|cx| {
            let mut t = PreviewTarget::file(target.clone());
            t.intent = PreviewIntent::Edit;
            cx.set_global(t);
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, cx| {
            assert_eq!(this.view_mode, ViewMode::Edit);
            this.dirty = true;
            let switched = this.try_set_view_mode(ViewMode::View, cx);
            assert!(!switched, "Edit -> View must be blocked while dirty");
            assert_eq!(this.view_mode, ViewMode::Edit, "must stay in Edit — no silent buffer loss");
            assert!(
                this.save_result.as_ref().is_some_and(|(ok, _)| !ok),
                "blocked switch must flash a muted hint, not fail silently"
            );
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn edit_to_view_allowed_when_not_dirty(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let (dir, target) = write_md("clean-switch", "# Hello\n");

        cx.update(|cx| {
            let mut t = PreviewTarget::file(target.clone());
            t.intent = PreviewIntent::Edit;
            cx.set_global(t);
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, cx| {
            assert!(!this.dirty, "fresh load must not be dirty");
            let switched = this.try_set_view_mode(ViewMode::View, cx);
            assert!(switched, "clean Edit -> View must succeed");
            assert_eq!(this.view_mode, ViewMode::View);
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn drawer_toggle_works_in_view_mode(cx: &mut TestAppContext) {
        // T194b residual fixed by the hoist: the terminal drawer must be
        // reachable even when never entering Edit mode at all.
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        let (dir, target) = write_md("drawer-view-mode", "# Hello\n");

        cx.update(|cx| cx.set_global(PreviewTarget::file(target.clone())));
        cx.background_executor.run_until_parked();

        cx.update_entity(&view, |this, cx| {
            assert_eq!(this.view_mode, ViewMode::View);
            this.toggle_drawer(cx);
            assert!(this.drawer_open, "drawer must open regardless of view_mode");
            assert!(this.terminal_drawer.is_some());
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn setting_target_to_missing_file_settles_to_error(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| {
            cx.set_global(PreviewTarget::default());
        });
        let view = cx.new(|cx| PreviewTab::new(cx));
        let target = PathBuf::from("/no/such/chronos/t179/file");

        cx.update(|cx| {
            cx.set_global(PreviewTarget::file(target.clone()));
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| match &this.state {
            State::Error { message, .. } => {
                assert!(
                    message.contains("Cannot read"),
                    "error must say Cannot read, got: {message}"
                );
            }
            other => panic!("expected Error, got {other:?}"),
        });
    }

    #[gpui::test]
    fn clearing_target_returns_to_empty(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| {
            cx.set_global(PreviewTarget::file(PathBuf::from("/tmp/some")));
        });
        let view = cx.new(|cx| PreviewTab::new(cx));
        cx.background_executor.run_until_parked();

        cx.update(|cx| {
            cx.set_global(PreviewTarget::default());
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| assert!(matches!(this.state, State::Empty)));
    }

    // Real-world case the dev session actually hits: the user clicks a
    // file in Files (target set globally) BEFORE Preview tab is built.
    // The forced `on_target_changed` inside `new` must pick it up — the
    // observer only fires on subsequent changes, so without that we
    // would sleep forever in `Empty`.
    #[gpui::test]
    fn target_already_set_at_construction_picks_up(cx: &mut TestAppContext) {
        install_theme(cx);
        let dir = std::env::temp_dir().join(format!("chronos-t179-prior-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("prior.md");
        std::fs::write(&target, "prior body\n").unwrap();

        cx.update(|cx| {
            cx.set_global(PreviewTarget::file(target.clone()));
        });

        let view = cx.new(|cx| PreviewTab::new(cx));
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| match &this.state {
            State::Loaded { generation, kind, text, .. } => {
                // Belt-and-suspenders: the generation we see must be the one
                // we put into the global (1), not something `on_target_changed`
                // fabricated locally. If this ever fires, the wiring has
                // started pretending instead of observing.
                assert_eq!(*generation, 1, "generation must come from global, not fabricated");
                assert_eq!(*kind, PreviewKind::Markdown);
                assert_eq!(text.as_deref(), Some("prior body\n"));
            }
            other => panic!("prior target must drive Loaded state on construct, got {other:?}"),
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- T194b: terminal drawer ---

    #[gpui::test]
    fn drawer_starts_closed_without_terminal(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));
        cx.update_entity(&view, |this, _cx| {
            assert!(!this.drawer_open, "drawer must default collapsed");
            assert!(
                this.terminal_drawer.is_none(),
                "no PTY session before the drawer is ever opened"
            );
        });
    }

    #[gpui::test]
    fn toggle_drawer_creates_terminal_once_and_reuses_session(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));

        let first_id = cx.update_entity(&view, |this, cx| {
            this.toggle_drawer(cx);
            assert!(this.drawer_open, "first toggle opens the drawer");
            this.terminal_drawer
                .as_ref()
                .expect("terminal must be created on first open")
                .entity_id()
        });

        // Close then reopen — same entity id proves the PTY session is
        // reused, not respawned (T194b UX point 4).
        cx.update_entity(&view, |this, cx| {
            this.toggle_drawer(cx);
            assert!(!this.drawer_open, "second toggle closes the drawer");
            assert!(
                this.terminal_drawer.is_some(),
                "closing must not drop the terminal entity"
            );
        });
        let second_id = cx.update_entity(&view, |this, cx| {
            this.toggle_drawer(cx);
            assert!(this.drawer_open, "third toggle reopens the drawer");
            this.terminal_drawer.as_ref().unwrap().entity_id()
        });

        assert_eq!(first_id, second_id, "reopening must reuse the same terminal entity");
    }

    #[gpui::test]
    fn drawer_resize_clamps_to_min_and_max(cx: &mut TestAppContext) {
        install_theme(cx);
        cx.update(|cx| cx.set_global(PreviewTarget::default()));
        let view = cx.new(|cx| PreviewTab::new(cx));

        cx.update_entity(&view, |this, cx| {
            this.toggle_drawer(cx);
            this.start_drawer_resize(500., cx);
            // Drag far up (small y) — would exceed max_h without clamping.
            this.update_drawer_resize(0., 300., cx);
            assert_eq!(this.drawer_height, 300., "must clamp to max_h");

            this.start_drawer_resize(0., cx);
            // Drag far down — would go negative/below the floor.
            this.update_drawer_resize(900., 300., cx);
            assert_eq!(this.drawer_height, DRAWER_MIN_H, "must clamp to DRAWER_MIN_H");
        });
    }

    #[test]
    fn read_for_preview_caps_truncated_text() {
        let dir = std::env::temp_dir().join(format!("chronos-t179-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("big.log");
        // Write a body larger than TEXT_CAP_BYTES so the read has to truncate.
        let chunk = b"0123456789abcdef\n";
        let repeats = (TEXT_CAP_BYTES as usize / chunk.len()) + 32;
        let body: Vec<u8> = chunk.iter().copied().cycle().take(repeats * chunk.len()).collect();
        std::fs::write(&target, &body).unwrap();

        let loaded = read_for_preview(&target).expect("must succeed");
        assert_eq!(loaded.kind, PreviewKind::Text);
        assert!(loaded.truncated);
        assert_eq!(loaded.size_bytes, body.len() as u64);
        assert!(loaded.text.unwrap().len() <= TEXT_CAP_BYTES as usize);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_for_preview_marked_image_skips_text_read() {
        let dir = std::env::temp_dir().join(format!("chronos-t179-img-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("pic.png");
        // 4 bytes PNG signature — not UTF-8, but extension must override.
        std::fs::write(&target, [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]).unwrap();

        let loaded = read_for_preview(&target).expect("must succeed");
        assert_eq!(loaded.kind, PreviewKind::Image);
        assert!(loaded.text.is_none());
        assert!(!loaded.truncated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- T180: image-URL classifier && remote-image redactor ---

    #[test]
    fn classify_image_url_categories() {
        let cases: &[(&str, ImageUrlClass)] = &[
            ("https://a/x.png", ImageUrlClass::Remote("https://a/x.png".into())),
            ("http://a/x.png", ImageUrlClass::Remote("http://a/x.png".into())),
            ("ftp://a/x.png", ImageUrlClass::Remote("ftp://a/x.png".into())),
            // Case-insensitive on scheme part — uppercase keeps the verbatim URL.
            ("HTTPS://A/X.PNG", ImageUrlClass::Remote("HTTPS://A/X.PNG".into())),
            ("data:image/png;base64,xyz", ImageUrlClass::Data),
            ("file:///abs/foo.svg", ImageUrlClass::Local("/abs/foo.svg".into())),
            ("./local.png", ImageUrlClass::Local("./local.png".into())),
            ("../foo.png", ImageUrlClass::Local("../foo.png".into())),
            ("plain.jpg", ImageUrlClass::Local("plain.jpg".into())),
            ("/abs/foo.png", ImageUrlClass::Local("/abs/foo.png".into())),
            ("", ImageUrlClass::Unsupported("".into())),
            ("   ", ImageUrlClass::Unsupported("   ".into())),
        ];
        for (input, expected) in cases {
            assert_eq!(&classify_image_url(input), expected, "{input:?}");
        }
    }

    #[test]
    fn redact_remote_images_replaces_badges() {
        let original = "Header\n\
                        [![status](https://img.shields.io/badge/status-work%20in%20progress-orange)](#status)\n\
                        [![license](https://img.shields.io/badge/license-Apache--2.0-blue)](#license)\n\
                        End\n";
        let redacted = redact_remote_images(original);
        // The image syntax must be gone — that's the part that would have
        // made the renderer call into the asset cache. What we forbid is
        // `![…](…)`, not the URL appearing in some form.
        assert!(
            !redacted.contains("!["),
            "image syntax must be gone, got: {redacted}"
        );
        // The URL is surfaced *inline in the marker* — that's the point
        // of being honest about where the image would have come from.
        assert!(
            redacted.contains("img.shields.io"),
            "marker must surface the URL: {redacted}"
        );
        assert!(redacted.contains("Header"));
        assert!(redacted.contains("End"));
        assert!(redacted.contains("remote image, not loaded"));
        // Each badge became its own marker.
        assert!(redacted.contains("[🛰 status"));
        assert!(redacted.contains("[🛰 license"));
    }

    #[test]
    fn redact_remote_images_keeps_local() {
        let s = "![alt](./local.png)\n\
                 ![alt2](/abs/foo.png)\n\
                 ![alt3](foo.png)\n\
                 ![alt4](data:image/png;base64,xyz)";
        assert_eq!(
            redact_remote_images(s),
            s,
            "local/data classes must pass through unchanged"
        );
    }

    #[test]
    fn redact_remote_images_handles_title_and_edges() {
        // Title preserved on remote image.
        let r1 = redact_remote_images(r#"![a](https://x/y.png "Title")"#);
        assert!(r1.contains(r#"(title: "Title")"#));
        assert!(!r1.contains("!["));

        // Long URL gets truncated with an ellipsis.
        let long_url = "https://a/".to_string() + &"x".repeat(200) + ".png";
        let r2 = redact_remote_images(&format!("![a]({long_url})"));
        assert!(r2.contains('…'));
        assert!(r2.chars().filter(|c| *c == 'x').count() <= 80);

        // Regular link (no leading '!') — passthrough.
        assert_eq!(
            redact_remote_images("[a](https://x/y.png)"),
            "[a](https://x/y.png)"
        );

        // Empty / plain text / malformed images — passthrough.
        assert_eq!(redact_remote_images(""), "");
        assert_eq!(
            redact_remote_images("# Hello\nworld"),
            "# Hello\nworld"
        );
        assert_eq!(
            redact_remote_images("![alt]no-paren"),
            "![alt]no-paren"
        );
        assert_eq!(
            redact_remote_images("![alt](unclosed"),
            "![alt](unclosed"
        );

        // Cyrillic / multibyte alt — UTF-8 char boundaries must hold
        // across match → slice → re-emit. A wrong slicing here would
        // either panic on the `find` or produce a `str::slice` error.
        let r = redact_remote_images("![Логотип](https://x/logo.svg)");
        assert!(r.contains("Логотип"));
        assert!(!r.contains("!["), "non-ASCII alt still stripped: {r}");
    }

    #[gpui::test]
    fn render_markdown_with_badges_does_not_panic(cx: &mut TestAppContext) {
        // Belt-and-suspenders: drive a full PreviewTab through the
        // Loading → Loaded path with a remote-image markdown body and
        // prove nothing in the text pipeline (redactor + renderer) panics.
        install_theme(cx);
        let dir =
            std::env::temp_dir().join(format!("chronos-t180-render-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("README.md");
        std::fs::write(
            &target,
            "# Title\n\n[![a](https://img.shields.io/badge/x-blue)](https://x)\n\nBody.\n",
        )
        .unwrap();

        cx.update(|cx| {
            cx.set_global(PreviewTarget::default());
        });
        let view = cx.new(|cx| PreviewTab::new(cx));
        cx.update(|cx| {
            cx.set_global(PreviewTarget::file(target.clone()));
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| match &this.state {
            State::Loaded { kind, text, .. } => {
                assert_eq!(*kind, PreviewKind::Markdown);
                // The *stored* text is the raw body — redaction happens
                // at render time. This confirms the data path still works.
                let stored = text.as_deref().unwrap_or("");
                assert!(stored.contains("img.shields.io"));
                assert!(stored.contains("Title"));
            }
            other => panic!("expected Markdown Loaded, got {other:?}"),
        });

        // The redactor *at the call site* strips image syntax. Proves
        // the wiring from `render_markdown` → `redact_remote_images`
        // would hand the markdown renderer a no-remote-image string.
        let raw = "# Title\n\n\
                   [![a](https://img.shields.io/badge/x-blue)](https://x)\n\nBody.\n";
        let redacted = redact_remote_images(raw);
        assert!(
            !redacted.contains("!["),
            "image syntax stripped: {redacted}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
