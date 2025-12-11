// Streaming info endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/streaming-info - Returns streaming endpoint metadata
export const GET = async () => {
  return await rpcCall('get_streaming_info', {});
};
