# T122 review — ACCEPTED WITH CAVEATS (2026-07-25)

**Report:** `report/T122-dev-shell-scripts-report.md`  
**Verdict:** **ACCEPTED WITH CAVEATS**

## Evidence

| Claim | Result |
|---|---|
| Five scripts in `scripts/dev/` + `install-dev-cli.sh` | ✅ |
| `~/.local/bin` symlinks → repo | ✅ re-checked |
| `pkill -x chronos` only | ✅ grep scripts |
| start single-instance | ✅ live: 2nd start exit 1 |
| stop idempotent | ✅ live |
| reload refuses release / no process | ✅ live messages match report |
| rebuild does not start/stop | ✅ script body |
| Design B reload documented | ✅ header |
| CONTRIBUTING Dev CLI | ✅ |
| REPO via walk / `CHRONOS_ROOT` | ✅ `common.sh` |
| ShellCheck «0 warnings» | ⚠️ **overstated** — without silencing SC2034, `common.sh` emits SC2034 on vars used by sourcers. Report used `-e SC1091` only; SC2034 remains. Non-blocking. |
| Date in report | ⚠️ typo **2025**-07-25 → should be 2026 |
| Commits | Report didn't commit — Architect commit `…` on accept |

## Live (Architect)

```
chronos-stop          # idempotent ok
chronos-start         # PID ok, log ~/.local/state/chronos/chronos.log
chronos-start         # fail single-instance
chronos-reload        # fail release correctly
chronos-stop          # ok
```

## Blocker found during accept (not script, but breaks rebuild)

`chronos-rebuild` failed on **E0382** `row_id` in `volume_popup/view.rs:591`
(`.id(row_id)` moved, then `.with_transition(row_id)`). Post-T121 regression
(working tree noise / transition API). **Errata:** `.id(row_id.clone())` —
`cargo check -p chronos` green after fix. Included in accept commit.

## Commit

```
54a54c0 scripts : chronos-{rebuild,reload,stop,start,debug} dev CLI (T122)
```

## Residual caveats

1. ShellCheck SC2034 noise on sourced vars — optional `export` or `# shellcheck disable=SC2034` in common.sh.
2. Report date year typo.
3. `chronos-debug` / full hot-reload path not re-smoked end-to-end on accept (would need long build); scripts look correct.
4. `REPO_CRATES` parsing in common.sh unused for behavior — dead-ish, not harmful.
