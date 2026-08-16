# T184 report

> **ПРИНЯТА 2026-08-02 архитектором (с эрратой).**
> Эррата §3: dock **запускает** apps — `bar/widgets/dock.rs:131-133`
> `launch(&entry.exec)`. Утверждение «декоративный / нет on_click» — ложь.
> Library всё равно использует тот же `crate::launcher::launch::launch`.
> Разделы 1–2, 4–9 сверены (96 desktop / 4 Game / 0 steam_app_*;
> rungameid; scene; GamingMode private; rail 7).

## 1 AppEntry / Categories

- `Categories=` is **not parsed**. `AppEntry` (`crates/services/src/applications/types.rs:6-18`) has fields `id, name, exec, icon, terminal` — no `categories`.
- `parse_desktop_file` (`types.rs:66-131`) reads `Type`, `Name`/`Name[lang]`, `Terminal`, `NoDisplay`, `Exec`, `Icon` only (`types.rs:93-101` match arms). `Categories` falls into the `_ => {}` catch-all, silently dropped.
- Minimal addition for a `Game` filter: add `pub categories: Vec<String>` to `AppEntry` (types.rs:7-18), parse `"Categories" => categories = value.split(';').filter(|s| !s.is_empty()).map(str::to_string).collect()` in the match at types.rs:93-101, default to `vec![]`. Pure addition, no signature break — `AppEntry` is constructed only in `parse_desktop_file` and test fixtures (`applications/mod.rs:262-269`, `launcher/search.rs:57-76`), so both need updating to keep compiling, but this is straight-line mechanical work, not migration risk.
- A separate pure filter fn (`services::applications::is_game_entry(&AppEntry) -> bool`, checking `categories.contains("Game")` OR id heuristics) belongs in `applications/mod.rs`, not `types.rs` — keeps parse/filter concerns separate as the plan's file map (§7) already specifies.

## 2 Machine counts (commands + numbers)

```
$ find /usr/share/applications ~/.local/share/applications -name '*.desktop' 2>/dev/null | wc -l
96

$ grep -lE '^Categories=.*Game' /usr/share/applications/*.desktop ~/.local/share/applications/*.desktop 2>/dev/null | wc -l
4

$ find /usr/share/applications ~/.local/share/applications -name 'steam_app_*.desktop' 2>/dev/null | wc -l
0

$ ls /usr/share/applications/steam*.desktop ~/.local/share/applications/steam*.desktop 2>/dev/null
/usr/share/applications/steam.desktop
(no ~/.local/share match — zsh "no matches found")

$ ls ~/.local/share/applications/ | grep -iE 'heroic|lutris|steam'
(empty)

$ ls ~/.local/share/flatpak/exports/share/applications 2>/dev/null
(dir does not exist)
$ ls /var/lib/flatpak/exports/share/applications 2>/dev/null | grep -i game
(dir does not exist)
```

**N games by Categories: 4** — but one of the 4 is `steam.desktop` itself (the Steam client launcher, `Categories=Network;FileTransfer;Game;`), not a playable game. The other 3 are real per-game shortcuts:

| id (filename stem) | Name | Exec |
|---|---|---|
| `steam` | Steam | `/usr/bin/steam %U` |
| `Counter-Strike 2` | Counter-Strike 2 | `steam steam://rungameid/730` |
| `PUBG BATTLEGROUNDS` | PUBG: BATTLEGROUNDS | `steam steam://rungameid/578080` |
| `SCUM` | SCUM | `steam steam://rungameid/513710` |

**N by steam_app_\*: 0.** The `steam_app_*` heuristic named in the plan (§2.3, §5.1) **does not match reality on this machine** — Steam-generated shortcuts here use the game's display name as the filename (`Counter-Strike 2.desktop`), not `steam_app_730.desktop`. The plan's fallback heuristic must key on the **Exec pattern** `steam steam://rungameid/<id>` (regex-extractable app id), not the filename.

**Real games without the heuristic: 3** (excluding the Steam client entry itself). **Library is not empty** with just `Categories=Game` filtering — it correctly surfaces CS2/PUBG/SCUM and also incorrectly surfaces the Steam client shell itself unless filtered out (id `== "steam"` or Exec not matching `rungameid`).

No flatpak install on this machine at all (`~/.local/share/flatpak` and `/var/lib/flatpak` don't exist — `flatpak` binary also not present per shell check). No heroic/lutris `.desktop` or binaries found.

## 3 Launch path

- **Real launch code**: `crates/app/src/launcher/launch.rs:14-30`, `pub fn launch(exec: &str) -> Result<()>`. Strips field codes defensively (`launch.rs:17`), then runs `setsid sh -c '<exec>'` with all stdio to `/dev/null` (`launch.rs:19-27`).
- Called from two sites, both in the launcher popup: `crates/app/src/launcher/view.rs:83` and `:194` (`launch(&entry.exec)` / `launch(&entry_for_click.exec)`).
- **Dock bar widget does NOT launch anything today.** `crates/app/src/bar/widgets/dock.rs` (502 lines) only resolves pin list → icon paths for display (`build_dock_icons`, dock.rs:206-228; `resolve_icon`, dock.rs:230-240). No `on_click`/`MouseDown`/`Command::new` anywhere in that file — grep confirms zero matches. `crates/app/src/dock/mod.rs` is 11 lines, module glue only. Pinned dock icons are decorative today, not click-to-launch.
- Steam Exec works through this path unmodified: `steam steam://rungameid/730` has no `%` field codes, `strip_field_codes` is a no-op on it, and `sh -c 'steam steam://rungameid/730'` is exactly how a shell would invoke it normally. **Same launcher path works, no new launch mechanism needed.** Verified by reading the Exec= line directly (§2 table above), not launched live.

## 4 Scene gaps

`crates/app/src/scene.rs`:

- `Scene` fields today (scene.rs:76-95): `id, name, mode, display, rail_tabs, active_tab, dock, extra` (flatten). No `kind`, `app`, `apply_gaming_profile`, or companion fields.
- No `activate` function exists — grep for `pub fn` in scene.rs shows only `find_by_id, resolve_last, filter_valid, current, rail_tabs_override, dock_override, active_tab_override, restore_for_mode, init`. Nothing writes `[last]`.
- `save_config` exists (scene.rs:138-154) but is `#[allow(dead_code)]` (scene.rs:137) — **unused**, comment at scene.rs:134-136 states explicitly it's reserved for "будущий SceneManager". `restore_for_mode` (scene.rs:256-279) is read-only on disk by design (comment scene.rs:250-253, code never calls `save_config`).
- New fields (`kind`, `app`, `apply_gaming_profile`, `audio_sink`, `microphone`, `hyprland_workspace`) can all be added as `#[serde(default)]` typed fields exactly like existing `display`/`rail_tabs`/`active_tab`/`dock` (scene.rs:81-91 pattern) — zero migration risk, `version` stays 1.
- Collision risk with `extra` flatten: **none currently**, because `extra` only captures keys the struct doesn't name. The moment a new named field (e.g. `app`) is added to the struct, any existing scenes.toml with an `app` key already sitting in someone's `extra` (there are none on this machine — checked scenes.toml doesn't exist yet, `load_config` falls to `ScenesConfig::default()` per scene.rs:130) would silently move from `extra` into the typed field on next parse. Not a real risk here since no user scenes.toml exists yet, but worth naming for T185: never repurpose a plan field name that's already floating in someone's `extra` blob in production.

## 5 GamingModeState

`crates/app/src/system_popup/gaming_mode.rs`:

- `apply` (gaming_mode.rs:97) and `revert` (gaming_mode.rs:125) are **private** (`fn`, no `pub`). Only `pub fn toggle` (gaming_mode.rs:88-95), `pub fn is_active` (gaming_mode.rs:77-79), `pub fn is_dnd` (gaming_mode.rs:81-83) are exported.
- Today only `toggle` is called, from the System popup UI: `crates/app/src/system_popup/view.rs:613` (`gaming_mode::toggle(cx)` inside the popup's click handler `gaming_mode_block`, view.rs:534). `GamingModeState::init(cx)` is called once at boot (`crates/app/src/system_popup/mod.rs:200`).
- **Yes**, `scene::activate` (once it exists, T185/T189) can call gaming mode **without touching the popup UI**: `apply`/`revert` are free functions in `gaming_mode.rs`, not methods gated by popup state — they only need `&mut App`. The plan's chosen approach (T189: export a minimal `pub(crate)` or `pub` wrapper, no UI changes) is consistent with what's on disk. Concretely: making `apply`/`revert` `pub(crate)` (module-visible) is enough — `scene.rs` and `gaming_mode.rs` are both under `crate::`, so `pub(crate)` avoids exposing internals crate-externally while unblocking T189.
- Success/fail log lines: `info!("gaming mode: hyprctl eval ON applied")` / `warn!("gaming mode: hyprctl eval ON failed: {e:?}")` (gaming_mode.rs:112-115), same pattern for power profile (gaming_mode.rs:117-120) and revert (gaming_mode.rs:141-148). Both operations run detached on `cx.background_spawn` (gaming_mode.rs:111,140) — fire-and-forget from the caller's perspective, state flips (`active`/`dnd`/`previous_profile`) happen synchronously before the async part (gaming_mode.rs:102-108, 133-138).
- **Confirmed via grep**: `workspace_mode.rs` has **zero** references to `gaming_mode`/`GamingModeState`. `workspace_mode::set` (workspace_mode.rs:176-193) touches only `WorkspaceModeState`, `scene::restore_for_mode`, config save, and `cx.refresh_windows()` — no compositor/power-profile call. The only call sites of `gaming_mode::*` in the whole tree are `system_popup/mod.rs:10,28,200` (module decl + init) and `system_popup/view.rs:21,66,126,534,613` (popup UI + toggle). §5 of the spec holds on current code.

## 6 Gamer rail today

`crates/app/src/side_panel_right/tabs.rs`:

- `PanelTab::for_mode(WorkspaceMode::Gamer)` (tabs.rs:427-440) returns exactly **7 tabs** in this order: `System, AcpSettings, McpSettings, LspSettings, ApiProviders, EditorSettings, HyprlandBinds`. Enforced by test `gamer_rail_stays_seven_tabs_without_new_work_tools` (tabs.rs:196-225) and `all_has_fourteen_tabs_in_fixed_order` catalog test (tabs.rs:13-34).
- Settings tail = 6 tabs, identical set/order to Developer's tail: verified by `developer_settings_group_matches_gamer_settings_group_order` (tabs.rs:227-248), which asserts `dev_settings == gamer[1..]`.
- Plan's insertion point (`System, Library, Scenes, Captures, <6 settings>`) is consistent with §2.5 of the approved plan and doesn't fight this invariant — new tabs go **between** `gamer[0]` (`System`) and `gamer[1..]` (settings tail), so the settings-order test keeps passing unmodified as long as T186 inserts before the settings slice, not inside it.

## 7 Pin/recent recommendation

**Recommend a separate `~/.config/chronos/games.toml`, not a section in `scenes.toml`.** Reasoning: `scenes.toml` today has exactly one writer path planned (`scene::activate`, T185) and one explicit rule (§4 constraint 3 in the plan) that `restore_for_mode` stays read-only — mixing pin/recent bookkeeping (which needs frequent small writes: pin toggle, recent-launch bump) into the same file multiplies the surface that must respect "don't corrupt on partial write" (T164 lesson) and couples two independently-evolving concerns (scene composition vs. library bookkeeping) into one serde struct and one `extra` flatten bucket. A dedicated `games.toml` with its own minimal schema (`pinned: Vec<String>`, `recent: Vec<{id, ts}>`) is a much smaller blast radius if it gets corrupted, and mirrors the existing `dock.toml` pattern (`crates/app/src/dock/config.rs`) already in the tree — same load/save/cache shape, nothing new to invent.

## 8 Out of scope confirmed

- **Game Deck overlay, gamepad service, controller focus** — no code found. Only reference in the entire tree: a comment in `workspace_mode.rs:41,47` about a nonexistent `icons/gamepad.svg` icon path (unrelated TODO from T159, not a feature).
- **Steam Web API, ProtonDB, achievements, playtime, artwork CDN** — zero references (`grep -rli` across `crates/app/src` and `crates/services/src` for these terms hits only `workspace_mode.rs` (gamepad comment, above) and `crates/services/src/aur/mod.rs` (unrelated: `opentelemetry` substring match in an AUR update-parser test string, coincidental token match, not gaming-related)).
- **Resolution/refresh control** — no display-service code found beyond `display` being a plain UUID string field on `Scene` (scene.rs:80-82), parsed/serialized only, never resolved to an actual output (per the scene.rs module doc, scene.rs:34-36).
- Everything above matches the plan's own "Нет" list (§2.2) and "Вне слайса" list (§3) — nothing contradicts it, nothing found unexpectedly present.

## 9 Risks for T185–T188 (short list)

1. **`steam_app_*` filename heuristic in the plan is factually wrong for this machine** — real Steam shortcuts use the display name as filename, with the app id embedded in `Exec=steam steam://rungameid/<id>`. T185/T187 must extract the id from Exec via regex/split on `rungameid/`, not from the filename. This is the single most important correction from this recon — building the heuristic as literally specified in plan §2.3 would match 0 games on this machine.
2. **Steam client's own `.desktop` (`steam.desktop`) also carries `Categories=Game`** — a naive `Categories contains Game` filter surfaces the Steam launcher itself as a "game" in the Library. T187 needs an explicit exclusion (id `== "steam"`, or better: Exec doesn't match `rungameid`/`heroic`/`lutris` launch pattern → treat as a game-adjacent app, not a game, or just needs a curated exclude list).
3. **Dock has no launch wiring at all today** (§3) — if T187/T188 assumed dock click-to-launch existed as a reusable primitive, it doesn't; Library tab must call `launcher::launch::launch()` directly (module is `pub(crate)`-reachable from `side_panel_right/`, needs `use crate::launcher::launch::launch` — check visibility of the `launcher` module from `side_panel_right`, may need `pub(crate)` bump on `launch.rs`'s `mod` declaration if it's currently private to `launcher/`).
4. **Zero flatpak, zero heroic/lutris on this dev machine** — Library smoke testing (T190 P2) will only ever show 3 real games (CS2/PUBG/SCUM) + whatever pins are added manually. Fine for smoke per plan §10 risk 1, but T190's "not empty" pass criterion should explicitly target these 3 ids, not assume any heuristic coverage beyond them.
5. **`GamingModeState::apply/revert` visibility bump is trivial** (`fn` → `pub(crate) fn`, gaming_mode.rs:97,125) but touches a file with `#[allow(unsafe_code)]`-adjacent discipline comments and a very specific hyprctl payload — T189 should NOT touch the payload constants or `toggle`, only add visibility + a new pub(crate) caller from scene.rs.

## Что НЕ сделано

- Games were **not launched live** — only `Exec=` lines were read (per task instructions, §3: "не запускай игру обязательно"). Launch-path mechanics were validated by code inspection (`launch()` behavior + confirmed no `%` field codes in the Steam Exec strings), not a live process spawn.
- Did not touch `~/.config/chronos/scenes.toml` — file doesn't exist on this machine yet (`load_config` returns default, scene.rs:130), so no live scene data was available to eyeball beyond the module's own doc-comment example.
- Did not check for a Steam flatpak install specifically (no flatpak install of any kind exists on this machine, so the question is moot here — Steam is the native/AUR package, confirmed by `/usr/share/applications/steam.desktop` + `/usr/lib/steam/steam.desktop`).
