-- Mixed Workload Load Test
-- Simulates realistic traffic with mix of static and dynamic routes
-- 60% static routes, 40% dynamic routes (realistic web traffic pattern)

local static_routes = {
    "/",
    "/health",
    "/api/status",
    "/metrics"
}

local dynamic_routes = {
    "/api/users/%d",
    "/api/posts/%d",
    "/api/products/%d"
}

local counter = 0
math.randomseed(os.time())

-- Initialize connection
function setup(thread)
    thread:set("id", counter)
    counter = counter + 1
end

-- Generate mixed request pattern
function request()
    counter = counter + 1
    local rand = math.random(100)

    -- 60% static routes
    if rand <= 60 then
        local idx = math.random(#static_routes)
        return wrk.format("GET", static_routes[idx], nil, nil)
    else
        -- 40% dynamic routes
        local idx = math.random(#dynamic_routes)
        local id = math.random(1000)
        local path = string.format(dynamic_routes[idx], id)
        return wrk.format("GET", path, nil, nil)
    end
end

-- Track static vs dynamic requests
local static_count = 0
local dynamic_count = 0

function response(status, headers, body)
    -- Simple heuristic: routes with numbers are dynamic
    if wrk.path:match("%d+") then
        dynamic_count = dynamic_count + 1
    else
        static_count = static_count + 1
    end
end

-- Print summary
function done(summary, latency, requests)
    io.write("-------------------------------------\n")
    io.write("Mixed Workload Load Test Results\n")
    io.write("-------------------------------------\n")
    io.write(string.format("Total Requests:    %d\n", summary.requests))
    io.write(string.format("Static Requests:   %d (%.1f%%)\n", static_count,
        (static_count / summary.requests) * 100))
    io.write(string.format("Dynamic Requests:  %d (%.1f%%)\n", dynamic_count,
        (dynamic_count / summary.requests) * 100))
    io.write(string.format("Duration:          %.2fs\n", summary.duration / 1000000))
    io.write(string.format("Requests/sec:      %.0f\n", summary.requests / (summary.duration / 1000000)))
    io.write(string.format("Avg Latency:       %.2fms\n", latency.mean / 1000))
    io.write(string.format("Max Latency:       %.2fms\n", latency.max / 1000))
    io.write(string.format("P50 Latency:       %.2fms\n", latency:percentile(50) / 1000))
    io.write(string.format("P90 Latency:       %.2fms\n", latency:percentile(90) / 1000))
    io.write(string.format("P99 Latency:       %.2fms\n", latency:percentile(99) / 1000))
    io.write("-------------------------------------\n")

    local rps = summary.requests / (summary.duration / 1000000)
    -- Mixed workload should achieve ~100k RPS (between static and dynamic targets)
    if rps >= 100000 then
        io.write("✅ PASS: Exceeded 100k RPS mixed workload target\n")
    else
        io.write(string.format("ℹ️  INFO: Got %.0f RPS (target 100k for mixed workload)\n", rps))
    end
    io.write("-------------------------------------\n")
end
