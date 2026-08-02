-- packaging/hyprland/30-autostart.lua
-- Starts ChronOS shell on session start. Prefer `chronos` on PATH (package)
-- or `chronos-start` (dev: always REPO/target/release/chronos after rebuild).
--
-- Do NOT hardcode a developer's project path in shipped profiles.

local function sh_ok(cmd)
    local f = io.popen(cmd .. " >/dev/null 2>&1; echo $?")
    local code = f and f:read("*n") or 1
    if f then f:close() end
    return code == 0
end

local function startChronosShell()
    if sh_ok("pgrep -x chronos") then
        return
    end
    -- Dev CLI first: resolves workspace release binary (freshest after rebuild).
    if sh_ok("command -v chronos-start") then
        os.execute("chronos-start >/dev/null 2>&1 &")
        return
    end
    -- Packaged binary on PATH.
    if sh_ok("command -v chronos") then
        os.execute("chronos >/dev/null 2>&1 &")
        return
    end
    hl.notification.create({
        text = "ChronOS: chronos not on PATH (install package or chronos-start)",
        timeout = 5000,
    })
end

hl.on("hyprland.start", function()
    startChronosShell()
end)

-- Reload does not kill chronos (single-instance IPC). Only ensure helper daemons
-- live elsewhere; shell stays up.
