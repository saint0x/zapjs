-- Dynamic Route Load Test
-- Validates the 45k RPS claim for dynamic routes with IPC
-- Tests routes with parameters that trigger TypeScript handlers

-- Array of dynamic route templates
local routes = {
    "/api/users/%d",
    "/api/posts/%d",
    "/api/products/%d",
    "/api/orders/%d",
    "/api/comments/%d"
}

local route_count = #routes
local counter = 0

-- Initialize connection
function setup(thread)
    thread:set("id", counter)
    counter = counter + 1
end

-- Generate request with rotating IDs
function request()
    -- Rotate through routes and IDs (1-1000)
    local route_idx = (counter % route_count) + 1
    local id = (counter % 1000) + 1
    counter = counter + 1

    local path = string.format(routes[route_idx], id)
    return wrk.format("GET", path, nil, nil)
end

-- Print summary
function done(summary, latency, requests)
    io.write("-------------------------------------\n")
    io.write("Dynamic Route + IPC Load Test Results\n")
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
    if rps >= 45000 then
        io.write("✅ PASS: Exceeded 45k RPS target\n")
    else
        io.write(string.format("❌ FAIL: Below 45k RPS target (got %.0f)\n", rps))
    end
    io.write("-------------------------------------\n")
end
