/**
 * Streaming Response Demo
 *
 * Demonstrates ZapJS streaming capabilities using async generators.
 * The runtime detects AsyncIterable responses and handles IPC streaming protocol.
 *
 * GET /api/stream - Returns Server-Sent Events (SSE) format stream
 */

// GET /api/stream - Streaming response demo
export const GET = async function* () {
  // Send initial event
  yield {
    data: `event: start\ndata: ${JSON.stringify({ type: 'start', timestamp: Date.now() })}\n\n`
  };

  // Stream 5 progress updates with delays
  for (let i = 1; i <= 5; i++) {
    // Simulate processing time
    await new Promise(resolve => setTimeout(resolve, 500));

    yield {
      data: `event: progress\ndata: ${JSON.stringify({
        type: 'progress',
        step: i,
        total: 5,
        message: `Processing step ${i} of 5...`,
        timestamp: Date.now()
      })}\n\n`
    };
  }

  // Send completion event
  yield {
    data: `event: complete\ndata: ${JSON.stringify({
      type: 'complete',
      message: 'All steps completed successfully!',
      timestamp: Date.now()
    })}\n\n`
  };
};
