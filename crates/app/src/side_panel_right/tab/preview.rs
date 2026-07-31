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
    AnyElement, Context, FontWeight, IntoElement, ObjectFit, ParentElement, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, img, prelude::*, px,
};

use crate::side_panel_right::preview_target::PreviewTarget;

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
    /// Holds the `observe_global` subscription. Dropping it removes the
    /// listener. `new` returns immediately after subscribing.
    _target_subscription: gpui::Subscription,
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
        };
        // The observer only fires on *changes*; the global may already
        // carry a path that was set before the tab was first created,
        // so we read the current value once.
        this.on_target_changed(cx);
        this
    }

    fn on_target_changed(&mut self, cx: &mut Context<Self>) {
        let (path, generation) = {
            let t = cx.global::<PreviewTarget>();
            (t.path.clone(), t.generation)
        };
        let Some(path) = path else {
            self.state = State::Empty;
            cx.notify();
            return;
        };
        self.state = State::Loading {
            generation,
            path: path.clone(),
        };
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let state = self.state.clone();
        let scroll = self.scroll.clone();
        match state {
            State::Empty => render_empty(&theme),
            State::Loading { path, .. } => render_loading(&path, &theme),
            State::Loaded {
                path,
                kind,
                size_bytes,
                text,
                truncated,
                ..
            } => render_loaded(&path, kind, size_bytes, text.as_deref(), truncated, &theme, &scroll),
            State::Error { message, .. } => render_error(&message, &theme),
        }
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
        PreviewKind::Markdown => render_markdown(text.unwrap_or(""), truncated, theme, scroll),
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
    // `gpui_component::text::markdown` takes `impl Into<SharedString>`.
    // `&str` has a direct into-impl (refcounted string) so we can pass the
    // borrowed `text` directly without paying for a `String` allocation.
    body.child(gpui_component::text::markdown(text))
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
}
