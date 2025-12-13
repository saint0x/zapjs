-- Static Route Load Test
-- Validates the 180k RPS claim for static route lookups
-- Tests simple routes with no parameters

-- Array of static routes to test
local routes = {
    "/",
    "/health",
    "/api/status",
    "/api/version",
    "/metrics",
    "/about",
    "/contact",
    "/api/config"
}

local route_count = #routes
local counter = 0

-- Initialize connection
function setup(thread)
    thread:set("id", counter)
    counter = counter + 1
end

-- Generate request
function request()
    -- Round-robin through routes
    local idx = (counter % route_count) + 1
    counter = counter + 1

    return wrk.format("GET", routes[idx], nil, nil)
end

-- Print summary
function done(summary, latency, requests)
    io.write("-------------------------------------\n")
    io.write("Static Route Load Test Results\n")
    io.write("-------------------------------------\n")
    io.write(string.format("Requests:      %d\n", summary.requests))
    io.write(string.format("Duration:      %.2fs\n", summary.duration / 1000000))
    io.write(string.format("Requests/sec:  %.0f\n", summary.requests / (summary.duration / 1000000)))
    io.write(string.format("Avg Latency:   %.2fms\n", latency.mean / 1000))
    io.write(string.format("Max Latency:   %.2fms\n", latency.max / 1000))
    io.write(string.format("P50 Latency:   %.2fms\n", latency:percentile(50) / 1000))
    io.write(string.format("P90 Latency:   %.2fms\n", latency:percentile(90) / 1000))
    io.write(string.format("P99 Latency:   %.2fms\n", latency:percentile(99) / 1000))
    io.write("-------------------------------------\n")

    local rps = summary.requests / (summary.duration / 1000000)
    if rps >= 180000 then
        io.write("✅ PASS: Exceeded 180k RPS target\n")
    else
        io.write(string.format("❌ FAIL: Below 180k RPS target (got %.0f)\n", rps))
    end
    io.write("-------------------------------------\n")
end
