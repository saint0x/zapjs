// WebSocket info endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/websocket-info - Returns WebSocket endpoint metadata
export const GET = async () => {
  return await rpcCall('get_websocket_info', {});
};
