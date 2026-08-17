# Kael Shader / Effect Port-Cost Audit — ChronOS (gpui-ce chronos edition)

**Scope:** READ-ONLY audit. Nothing in either repo was modified.
**Date:** 2026-07-16
**Donor:** `ChronOS/reference/kael-main` (Apache-2.0, Kael fork of GPUI)
**Target:** `Source/` — backend crate `gpui_wgpu` (WGSL, wgpu) + platform `gpui_linux` + main crate `gpui`
**Verdict headline:** Both backends are WGSL, so copying *math* is easy. But the two shaders differ in bind-group conventions, instance structs, and feature surface. Our fork is a *subset* of Kael's Linux/Blade pipeline: we already have erf box shadows; we lack gradient borders, color filters, backdrop blur, and effect layers entirely.

---

## 0. Pipeline compatibility (the crux) — VERIFIED

### Bind group / instance conventions differ

| Concern | Kael `blade/shaders.wgsl` | Our `gpui_wgpu/src/shaders.wgsl` |
|---|---|---|
| Globals | `var<uniform> globals` + separate `var<uniform> gamma_ratios`, `var<uniform> grayscale_enhanced_contrast` (no explicit `@group`/`@binding` — blade resolves) | `@group(0) @binding(0) globals: GlobalParams`; `@group(0) @binding(1) gamma_params: GammaParams` (explicit) |
| Sprite texture/sampler | module-scope `var t_sprite: texture_2d<f32>; var s_sprite: sampler;` (implicit binding) | `@group(1) @binding(1) t_sprite`; `@group(1) @binding(2) s_sprite` (explicit) |
| Instance storage | `var<storage, read> b_quads: array<Quad>;` (implicit group) | `@group(1) @binding(0) var<storage, read> b_quads` |
| Struct field order | extra `transform`, `blend_mode`, `color_filter`, `rounded_clip_*` fields on Quad/Shadow | simpler structs, no transform/blend/color_filter/rounded-clip fields |

**Consequence:** Kael's WGSL is not copy-as-is compilable under our `wgpu`/`naga` pipeline because (a) it uses implicit blade bind groups whereas we require explicit `@group`/`@binding`, and (b) our `GammaParams` uniform layout would have to be reconciled with Kael's split uniforms. The *fragment/vertex math* is portable; the *plumbing* (struct layout, bindings, instance-buffer feed) must be adapted.

### Instance feed path
- Kael: `blade_renderer.rs` `draw_blur_rects` (line 1477), builds `BlurPass` via `BlurPass::horizontal/composite` (lines 146–168), uploads via `instance_belt.alloc_typed` → `ShaderBlurData{globals, t_sprite, s_sprite, b_blurs}`.
- Ours: `wgpu_renderer.rs` `draw_quads/draw_shadows` (lines 1342–1372) call `instance_bytes()` (raw `repr(C)` transmute) → `draw_instances` → `WgpuPipelines`. Our `pipelines` struct (search) has **no blur pipeline** — only `quads, shadows, underlines, mono_sprites, subpixel_sprites, poly_sprites, path_rasterization, paths`. **Backdrop blur does not exist in our fork.**

---

## 1. Erf-based analytic box shadows

**Donor (WGSL):** `crates/kael/src/platform/blade/shaders.wgsl`
- `erf(v: vec2<f32>)` — line 317
- `blur_along_x(...)` — line 325 (analytic gaussian integral via erf)
- `gaussian(x, sigma)` — line 311
- `struct Shadow` — lines 1186–1199 (`color_filter: ColorFilter` field present, lines 1197)
- `vs_shadow` — line 1209; `fs_shadow` — line 1228 (loop 4 samples, inset path, `apply_color_filter` at 1262, `rounded_clip_factor` at 1260)
- Shader lang: **WGSL only** (Linux path). Metal mirror in `shaders.metal` exists but irrelevant to ChronOS.
- Approx LOC (shader side): ~160.

**Donor (Rust):** `crates/kael/src/style.rs` `struct BoxShadow` (lines ~330–375, `blur_radius`/`spread_radius`/`inset`); `scene.rs` `Shadow` primitive (border_color is `Background`); `window.rs` shadow emission (~5560–5680) threads `color_filter` into the instance.

**Our-fork analog:** **PRESENT (functional).** `Source/gpui/src/scene.rs` `struct Shadow` (lines 540–552, with `element_bounds`/`element_corner_radii` for inset); `Source/gpui_wgpu/src/shaders.wgsl` `erf` (325), `blur_along_x` (333), `gaussian` (319), `struct Shadow` (951–966), `vs_shadow` (977), `fs_shadow` (1001) — same 4-sample analytic erf math.

**Portability verdict:** **COPY-AS-IS (no port needed).** We already implement the identical erf box-shadow algorithm. The only delta: Kael's Shadow carries a `color_filter` + `rounded_clip` fields our Shader struct lacks — but those belong to effects #4/#5, not the shadow core.
**Cost:** **S** (none — already shipped; only cross-wiring if color-filter support added).

---

## 2. Two-pass separable backdrop blur

**Donor (WGSL):** `crates/kael/src/platform/blade/shaders.wgsl`
- `struct BlurPass` — lines 1074–1082 (`target_bounds`, `sample_bounds`, `clip_bounds`, `corner_radii`, `tint: Hsla`, `blur_radius`, `saturation`)
- `blur_along_axis` — lines 1100–1126. **16-tap clamp confirmed:** line 1103 `let radius = min(i32(ceil(blur.blur_radius * 3.0)), 16);` then loop `for (var offset = -16; offset <= 16; offset++)`.
- `composite_blur` — lines 1128–1139 (grayscale/saturation/tint compositing, `GRAYSCALE_FACTORS`)
- `vs_blur` — 1142; `fs_blur_horizontal` — 1154; `fs_blur_composite` — 1163
- Shader lang: **WGSL only** (Linux). Metal mirror in `shaders.metal`.
- Approx LOC (shader): ~170.

**Donor (Rust):** `blade_renderer.rs`
- `struct BlurPass` (Rust) — lines 135–143; `horizontal`/`composite` — 146–168
- `draw_blur_rects` — lines 1477–1593: copies capture region into `blur_source_texture`, runs `blur_horizontal` pass (full-res, into `blur_horizontal_texture`), then `blur_composite` pass (load into target). Two passes per rect, **no downsampling** (GAP_ANALYSIS §5 "16-tap + per-rect full-res" confirmed).
- `BlurRect` emitted from `window.rs` (`tint`, `saturation` at 6364–6399) feeding the `PrimitiveBatch::BlurRects` path (renderer 841, 1167).

**Our-fork analog:** **ABSENT.** No `BlurPass`, no `blur_horizontal`/`blur_composite` pipelines, no `draw_blur_rects`, no `BlurRect` primitive. `gpui/src` has zero backdrop-blur plumbing (only `BoxShadow.blur_radius`). `wgpu_renderer.rs` pipeline set has no blur entry.

**Portability verdict:** **ADAPT (NOT copy-as-is).** The WGSL math is reusable but must be (a) given explicit `@group`/`@binding` + reconciled with our `GammaParams`/`globals` layout, and (b) backed by new Rust: a `BlurRect`/`BlurPass` `Scene` primitive, `paint_blur` on `Window`, an intermediate `blur_source`/`blur_horizontal` texture pair in `wgpu_renderer`, two new pipelines, and frame-graph wiring for the offscreen capture. Our instance-upload path (`instance_bytes` raw transmute) can reuse `repr(C)` `BlurPass` but only after the bind-group refactor.
**Cost:** **L.** (WGSL port small; Rust renderer + texture intermediates + pipeline + scene primitive + window API + frame-graph is the bulk.)

---

## 3. Gradient borders (`border_gradient`)

**Donor (WGSL):** `crates/kael/src/platform/blade/shaders.wgsl`
- `struct Background` carries `colors: array<LinearColorStop, 8>` + `stop_count` (lines 106–117)
- `prepare_gradient_color` — lines 431–458 (builds up to 4 varyings, but fragment-side `interpolate_multi_stop` loop reads all 8 stops)
- `interpolate_multi_stop` — lines 471–504 (dynamic-loop fragment evaluation, the 8-stop mechanism)
- In `fs_quad`: gradient border evaluated at lines 802–811 via `prepare_gradient_color(quad.border_color...)` + `gradient_color(quad.border_color, ...)` — i.e. the border is a full `Background`, not a flat `Hsla`.
- Approx LOC (shader): ~80 relevant.

**Donor (Rust):** `crates/kael/src/style.rs` `border_gradient: Option<Background>` (line 252); `crates/kael/src/styled.rs` `border_gradient(...)` builder (line 594); `crates/kael/src/scene.rs` `Shadow`/`Quad` border_color is `Background` (scene.rs:877, window.rs:9344 `border_color: Background`); `div.rs` `border_color` becomes a `Background` (window.rs:9373 `impl Into<Background>`). So a border can be a gradient.

**Our-fork analog:** **PARTIAL / DIFFERENT TYPE.** `Source/gpui/src/scene.rs` `Quad.border_color: Hsla` (line 507) — a single flat color, **not** a `Background`. `Source/gpui/src/color.rs` `Background.colors: [LinearColorStop; 2]` (line 784) — 2-stop only. No `border_gradient` builder; `border_color` takes `impl Into<Hsla>`.

**Portability verdict:** **ADAPT.** Requires (a) widening `Background.colors` from `[_;2]`→`[_;8]` (shared with effect #3a multi-stop), and (b) changing `Quad.border_color` from `Hsla` to `Background` + adding `border_gradient` plumbing in `style.rs`/`styled.rs`/`div.rs`, plus the shader's fragment-side `interpolate_multi_stop` loop (currently our `gradient_color` only handles 2 stops via varyings `color0/color1`). The 8-stop fragment loop is the expensive shader part.
**Cost:** **S–M.** Border wiring is small; the real cost is the shared 8-stop gradient upgrade (see below) it depends on.

---

## 4. Color filters (4-channel `ColorFilter`)

**Donor (WGSL):** `crates/kael/src/platform/blade/shaders.wgsl`
- `struct ColorFilter { grayscale, saturate, brightness, contrast: f32 }` — lines 141–146
- `apply_color_filter(color, cf)` — line 360 (composes the 4 ops)
- Applied in `fs_quad` (line 693, fast path), `fs_shadow` (1262), sprite shaders (1015, 1589, 1604, 1620, 1647) — every quad/sprite/shadow instance carries a `color_filter: ColorFilter` field.
- Shader lang: **WGSL only** (Linux).
- Approx LOC (shader): ~15 core + ~10 call sites.

**Donor (Rust):** `crates/kael/src/window.rs` `element_color_filter: ColorFilter` (3288, init 3734), `ColorFilter::identity`/`compose`, `with_color_filter` (5871); threaded into quads/shadows/sprites (lines 5616, 5654, 5684, 6338, 6427, 6497, 6531, 6638, 6659, 6731, 6803, 6855, 6909) and `paint_cached_surface`/`paint_effect_surface`. `ColorFilter` type lives in kael root (`color.rs`/re-export).

**Our-fork analog:** **ABSENT.** No `ColorFilter` type anywhere in `gpui/src` (search: 0 hits). `Quad`/`Shadow`/`Sprite` WGSL structs have no `color_filter` field; our `fs_quad` fast path lacks `apply_color_filter`. `wgpu_renderer` does not upload any filter data.

**Portability verdict:** **ADAPT.** The WGSL `apply_color_filter` is trivially portable (add the struct + call sites). The work is: (a) define `ColorFilter { grayscale, saturate, brightness, contrast }` in `gpui/src/color.rs`, (b) add `color_filter: ColorFilter` to `Quad`/`Shadow`/`PolychromeSprite`/`MonochromeSprite` scene structs (changes `repr(C)` layout → must keep naga layout test green), (c) thread `element_color_filter` through `window.rs` paint paths, (d) add `with_color_filter` builder, (e) extend `wgpu_renderer` to upload the field. Note GAP_ANALYSIS §"Visual polish" says hue/invert/sepia are *missing* — only 4 channels exist; don't overstate.
**Cost:** **M.** Spread across many call sites + layout-sensitive struct edits, but each change is small.

---

## 5. Effect layers (`effect_layer`)

**Donor (Rust, element):** `crates/kael/src/elements/effects.rs` (entire file, 330 LOC)
- `effect_layer(child)` builder (line 23); `EffectLayer` struct (34) with `content_blur: Pixels`, `drop_shadow: Option<BoxShadow>`
- `paint` (157) → `cached_paint` → `window.paint_effect_surface(surface, content_blur, drop_shadow)` (176)
- Renders subtree to an **offscreen `CachedSurface`** tile, then composites: content blur via a `PolychromeSprite` with `blur_radius`/`sprite_kind = CONTENT_BLURRED`, and/or a drop shadow via `CONTENT_SHADOW` sprite (render_tests 288–329).
- **No dedicated shader** — it reuses the blur pipeline (#2) for content blur and the shadow pipeline (#1) for the silhouette. The "effect" is a **compositing strategy** (offscreen tile + sprite), not a new shader entrypoint.

**Donor (Rust, backend):** `window.rs` `paint_effect_surface` (5633) builds `PolychromeSprite { blur_radius, sprite_kind: CONTENT_BLURRED|CONTENT_SHADOW, color_filter, ... }` (5616 etc.); requires `cache.rs` `CachedSurface` machinery.

**Our-fork analog:** **ABSENT.** No `effect_layer`, no `paint_effect_surface`, no `CONTENT_BLURRED`/`CONTENT_SHADOW` sprite kinds, no `content_blur` concept. Our `PolychromeSprite` has no `blur_radius` field. `gpui/src/elements/` has no `effects.rs`.

**Portability verdict:** **ADAPT / BLOCKED on #2.** The `EffectLayer` element is pure Rust and portable in principle, but it *depends on* backdrop/content blur (#2) and `CachedSurface` offscreen rendering. Our fork lacks both the blur shader pipeline and (need to verify) the offscreen-surface compositing path `effect_layer` leans on. Porting `effects.rs` alone yields a no-op until #2 lands.
**Cost:** **M** for the element (straight Rust port) **+ the L from #2 it requires.** Effectively **L** end-to-end because blur is a prerequisite.

---

## 6. GAP_ANALYSIS.md shader claims — VERIFIED vs NOT

| CLAIM (KAEL_GAP_ANALYSIS.md) | Verdict | Evidence |
|---|---|---|
| §3a "analytic erf-based GPU box shadows (Figma-grade)" | **CONFIRMED** | `shaders.wgsl` erf 317, blur_along_x 325; our fork matches |
| §4 "Backdrop-blur kernel clamped to 16 taps at full res" (`radius = min(ceil(sigma*3), 16)`; claims `shaders.metal:497`) | **CONFIRMED in WGSL** at `shaders.wgsl:1103` (metal line not checked, irrelevant) | 16-tap clamp + per-rect full-res passes present; no downsample/mip path |
| §8 "Multi-stop gradients up to 8 stops (was 4)... fragment-side loop... `Background.colors` now `[LinearColorStop; 8]`" | **CONFIRMED** | `shaders.wgsl` Background 106–117 `colors: array<LinearColorStop,8>` + `stop_count`; `interpolate_multi_stop` 471–504; `color.rs:697` `[LinearColorStop; 8]` |
| §8 "All six blend modes read the backdrop" (Multiply/Screen fixed-function; Overlay/SoftLight/Difference framebuffer-fetch `quad_fragment_blend`) | **NOT CONFIRMED in WGSL — likely Metal-only** | In `blade/shaders.wgsl` there is **no** `quad_fragment_blend`, no `[[color(0)]]` framebuffer-fetch, no `DestinationColor`/`Zero`/`OneMinusSourceColor` pipeline. Only `apply_blend_mode` (639–665) doing `src*src`/`1-(1-src)^2`/etc. — i.e. the *old approximation*. GAP_ANALYSIS itself admits "blade/directx keep their current approximation (no regression)" (§8 line 364). **So the 6-mode backdrop-read fix is Metal-only; the Linux/WGSL path we care about does NOT have it.** |
| §3a "4-channel color filter" | **CONFIRMED (WGSL)** | `ColorFilter` 141–146, `apply_color_filter` 360, threaded everywhere |
| §3a "backdrop blur+saturate" | **CONFIRMED (WGSL)** | `composite_blur` 1128 + `saturation` field |
| §4 "no downsampling/batching" of blur | **CONFIRMED** | `draw_blur_rects` 1497 loops per-rect, no mip/downsample |

**Unverifiable / flag:** GAP_ANALYSIS §8 states the 8-stop + blend fixes are done "in **both** shader backends — Metal + blade." The WGSL 8-stop claim is true. The **blade WGSL blend-mode fix is false** — the WGSL still carries only the approximation. Anyone porting "blend modes" from Kael-Linux would get the *buggy* approximation, not the fixed Metal behavior. State this gap explicitly.

---

## 7. Per-effect cost summary

| # | Effect | Donor files (WGSL / Rust) | Shader lang | Our-fork analog | Verdict | Cost |
|---|---|---|---|---|---|---|
| 1 | Erf box shadow | `blade/shaders.wgsl:311–328,1186–1264` / `style.rs` BoxShadow, `scene.rs` Shadow, `window.rs:5560–5680` | WGSL | **PRESENT, identical math** | copy-as-is (none) | **S** |
| 2 | Backdrop blur (2-pass) | `blade/shaders.wgsl:1074–1182` / `blade_renderer.rs:135–168,1477–1593`; `window.rs:6364–6399` | WGSL | **ABSENT** | adapt (new pipeline+tex+primitive) | **L** |
| 3 | Gradient borders | `blade/shaders.wgsl:106–117,431–504,802–811` / `style.rs:252`, `styled.rs:594`, `scene.rs:877`, `window.rs:9344` | WGSL | partial (flat `Hsla` border, 2-stop bg) | adapt (needs 8-stop upgrade) | **S–M** |
| 4 | Color filter (4-ch) | `blade/shaders.wgsl:141–146,360` + call sites / `window.rs:3288,5871` | WGSL | **ABSENT** | adapt (new type + struct fields + threading) | **M** |
| 5 | Effect layers | `elements/effects.rs` (330 LOC) / `window.rs:5633` `paint_effect_surface` | Rust (no new shader) | **ABSENT** (blocked on #2) | adapt (needs #2 + offscreen) | **L** (M element + L blur) |

**Total program:** 1×S (shadow, already done) + 1×S–M (gradient border) + 1×M (color filter) + 1×L (blur) + 1×L (effect layer, gated on blur) ≈ **dominated by the L items**. Blur (#2) is the linchpin — effects #3 (8-stop) and #5 both ride on it.

---

## 8. Concrete integration points (where to land the port in `Source/`)

**Shader edits — `gpui_wgpu/src/shaders.wgsl`:**
- #2: add `struct BlurPass` (mirror 1074–1082), `blur_along_axis`/`composite_blur`/`vs_blur`/`fs_blur_horizontal`/`fs_blur_composite` (1100–1182), with explicit `@group(1) @binding(…)` + reconcile `globals`/`GammaParams` (80–97). Add `b_blurs` storage binding.
- #3: widen `Background` to 8 stops + add `interpolate_multi_stop` loop (471–504); change `gradient_color` to fragment-side evaluation; change `Quad.border_color` to a `Background`; add border-gradient branch in `fs_quad` (802–811).
- #4: add `struct ColorFilter` (141–146) + `apply_color_filter` (360); add `color_filter` field to `Quad`/`Shadow`/`PolychromeSprite` structs; call in `fs_quad` (693) / `fs_shadow` (1262) / sprite shaders.

**Renderer edits — `gpui_wgpu/src/wgpu_renderer.rs`:**
- #2: add `blur_source_texture` + `blur_horizontal_texture` intermediates, `WgpuPipelines.blur_horizontal`/`blur_composite`, `create_pipelines` entries (near 730–868), and `draw_blur_rects` (mirror 1477–1593) fed from a new `Scene` `BlurRect` batch.
- #4: extend `draw_quads`/`draw_shadows`/`draw_polychrome_sprites` to upload the new `color_filter` field (already via `instance_bytes` transmute — layout must stay naga-verified).

**Core Rust edits — `gpui/src`:**
- `color.rs`: add `ColorFilter` (#4); widen `Background.colors` to `[LinearColorStop;8]` + `stop_count` (#3).
- `scene.rs`: `Quad.border_color: Background` (#3); add `BlurRect`/`BlurPass` primitive (#2); add `color_filter` to `Quad`/`Shadow`/`PolychromeSprite` (#4).
- `style.rs`/`styled.rs`/`elements/div.rs`: `border_gradient` builder + `border_color: Into<Background>` (#3); `color_filter` Style field + `with_color_filter` (#4).
- `window.rs`: `paint_blur`/`paint_effect_surface` (#2/#5); thread `element_color_filter` (#4); `content_blur`/`drop_shadow` emission (#5).
- `elements/effects.rs`: **new file** ported from Kael `effects.rs` (#5) — but non-functional until #2 lands.

**Tests to mirror:** Kael `effects.rs` render_tests (288–329), 8-stop golden `headless_render.rs` (483–628), `color.rs` gradient-stop tests (1027–1047).

---

## 9. Gaps / cautions (explicit)
- **Blend-mode fix is Metal-only.** Do NOT assume Kael-Linux WGSL has backdrop-reading blend modes; it does not (only `apply_blend_mode` approximation). Porting "blend modes" from the donor's WGSL gives you the known-wrong behavior.
- **Hue/invert/sepia are NOT in `ColorFilter`** (GAP_ANALYSIS admits this) — it is exactly 4 channels (grayscale/saturate/brightness/contrast). Don't scope creep.
- **8-stop gradient is the real shared cost** for #3 — once `Background` is widened, gradient *fills* and gradient *borders* both benefit. Budget it once.
- **Effect layers depend on blur (#2) and offscreen `CachedSurface` compositing** — verify our fork's `cache.rs`/`CachedSurface` supports offscreen tile → sprite compositing before porting `effects.rs`, or it will be inert.
- All cost letters assume a developer fluent in wgpu/naga; the WGSL math is low-risk, the `repr(C)` layout + bind-group reconciliation is where regressions hide (keep Kael's naga layout test green).
