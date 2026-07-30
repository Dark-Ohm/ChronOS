# T123 review — ACCEPTED WITH CAVEATS (2026-07-25)

**Report:** `report/T123-audio-volume-drag-coalesce-report.md`  
**Verdict:** **ACCEPTED WITH CAVEATS**

## Evidence

| Claim | Result |
|---|---|
| Volume fast-path: no full `read_state` / no pw-dump | ✅ `dispatch` Set*Volume → watch only |
| Optimistic `data.set` before wpctl | ✅ |
| Light confirm `get-volume` only + merge preserves devices | ✅ `read_volume_only` + `merge_volume_into_state` |
| Non-volume commands full re-read | ✅ |
| UI throttle 32ms / 1% | ✅ |
| Optimistic thumb (`thumb_fill_w`) | ✅ fill = service, thumb = dispatched |
| Source unmute uses `ToggleSourceMute` | ✅ |
| Unit tests audio 27/27 | ✅ re-run accept |
| Commits by minion | ❌ uncommitted — Architect committed |

## Critical errata (Architect)

**Coalesce loop spin:** original `run_volume_coalesce` after first `Some(pv)` never waited on a *new* notification — `while is_none` was false forever, re-applied the same pending every 5 ms → continuous `wpctl set-volume`.

**Fix in accept commit:** wait on `rx.changed()`, apply, drain `has_changed` backlog, then wait again. `PendingVolume: PartialEq` for watch.

**Throttle paint:** throttled path now still updates `dispatched_vol` + `notify` so thumb tracks without service spam.

## Commit

```
5cad0bb audio : volume drag coalesce + light re-read (T123)
```

## Residual caveats

1. Live drag smoke not architect-grimmed — user should feel slider after restart.
2. Unmute still separate `Toggle*Mute` during muted drag (report recommendation #1).
3. `chronos` full check may fail on unrelated toast WIP — T123 zone green via services tests.
4. Report “Reload design B” noise — copy-paste from T122, ignore.
5. Float `PartialEq` on `PendingVolume.volume` — identical re-sends may not wake watch (fine for drag).

## Accept checklist (brief)

- [x] No pw-dump on volume set  
- [x] Coalesce latest-wins (after errata)  
- [x] Optimistic state + UI thumb  
- [x] Unit tests  
- [ ] Live hard-drag (user)  

## Errata 2 (compile / ship)

Release build failed: `Context::read` / private `Context::update` on
`VolumePopupView` — wrong GPUI API. Fixed to pass `&mut VolumePopupView` in
listeners and `drag_preview` into `endpoint_block`. Commit after `5cad0bb`.
WIP `crates/app/src/toast/` was unwired from main (left untracked) so T124
draft does not block release.
