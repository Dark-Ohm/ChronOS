chronos.bar:register({
    name = "clock",
    section = "left",
    render = function()
        local now = chronos.time.now()
        local hours = math.floor((now % 86400) / 3600)
        local minutes = math.floor((now % 3600) / 60)
        local seconds = math.floor(now % 60)
        local time_str = string.format("%02d:%02d:%02d", hours, minutes, seconds)

        return {
            type = "text",
            content = time_str,
            style = { color = "#ffffff", size = 14 }
        }
    end,
    on_tick = function(callback)
        chronos.on("tick", callback)
    end
})

chronos.log.info("clock plugin loaded")
