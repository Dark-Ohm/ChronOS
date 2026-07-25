# Live-smoke recipe for ChronOS on Wayland (Hyprland)

Verifying the desktop shell means observing REAL Wayland surfaces — there is no
headless mode. Use this copy-paste flow. It was hardened during the HERMES №4
notifications-popup work (2026-07-17) after a wide `pkill` made a working
daemon look like it crashed.

## 0. Pre-flight
- Confirm no live instance (zombies don't count): `ps -eo pid,comm | grep chronos`
  — ignore `<defunct>`. If a REAL one is owned by you and you must restart it,
  `kill <that-pid>` (never a pattern kill).
- Stale IPC socket self-heals: `crates/app/src/ipc/service.rs::acquire_at` removes
  a dead `/run/user/1000/chronos.sock` and binds. Just launch; don't `rm` first.
- Know the layout: `hyprctl monitors -j` → here `DP-1` 2560x1440 at (0,0),
  `HDMI-A-1` 1920x1200 at (2560,0). The popup targets the primary (DP-1).

## 1. Launch ONE controlled instance
```bash
cd /home/neo/projects/chronos-ecosystem/ChronOS
RUST_LOG=info nohup target/debug/chronos >/tmp/chronos_live.log 2>&1 &
echo $! > /tmp/chronos_pid
sleep 3
ps -p "$(cat /tmp/chronos_pid)" -o pid,stat,comm   # expect S<sl, not Z
busctl --user list | grep -i notification            # expect YOUR pid owns name
```

## 2. Verify a layer-shell popup
Popups are overlay-layer, NOT clients. Check `hyprctl layers`, not `hyprctl clients`.
```bash
notify-send -u critical "Alarm" "Backup failed"
sleep 1
# layer present?
hyprctl layers -j | python3 -c "import sys,json;d=json.load(sys.stdin);f=any('notifications' in str(s) for m in d.values() for g in m.values() if isinstance(g,dict) for ns,surfs in g.items() if isinstance(surfs,list) for s in surfs);print('PRESENT' if f else 'NONE')"
# screenshot + crop (Top|Right, 360x96, 12px margin on DP-1 -> x=2188 y=44)
grim -o DP-1 /tmp/A_full.png
python3 -c "from PIL import Image; Image.open('/tmp/A_full.png').crop((2188,44,2188+360,44+96)).save('/tmp/A_popup.png')"
# then vision_analyze /tmp/A_popup.png  -> confirm card + red urgency border
```

## 3. Expire
```bash
notify-send -t 3000 -u normal "Timer" "gone in 3s"; sleep 1
hyprctl layers -j | ...   # PRESENT
sleep 3
hyprctl layers -j | ...   # GONE (empty stack -> sync_window removes surface)
```

## 4. Close (convergent D-Bus path == X button)
```bash
notify-send -u normal "Closable" "x"; sleep 1
# correct dbus-send syntax uses a COLON: uint32:1  (uint32 1 is REJECTED)
dbus-send --session --dest=org.freedesktop.Notifications \
  /org/freedesktop/Notifications org.freedesktop.Notifications.CloseNotification uint32:1
sleep 1
hyprctl layers -j | ...   # GONE -> close wiring proven
```

## 5. Action button (no click injection available)
There is NO Wayland input tool installed (ydotool/dotool absent; xdotool is
X11-only), so a literal mouse click on the action button can't be injected.
Verify the `dispatch(InvokeAction)` path with a UNIT TEST instead — the button
closure is structurally identical to X. See `crates/services/src/notification/
mod.rs::invoke_action_closes_notification` for the pattern (bogus key ignored,
matching key closes).

## 6. Benign noise — do NOT chase
- `ERROR: window not found` pairs correlate with `network`-subscriber reconnect
  timeouts and the IPC rapid-toggle race, NOT with popup bugs. A process logging
  these yet surviving the whole smoke (S<sl) is healthy.
- Pre-existing build warnings you didn't introduce (`ContentMask`, unused `Task`)
  are not yours — leave them.

## 7. Shut down
```bash
kill "$(cat /tmp/chronos_pid)"   # ONLY your PID; never pkill -f target/debug/chronos
```
