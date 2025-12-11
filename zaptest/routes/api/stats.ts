// Stats endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/stats - Returns site statistics
export const GET = async () => {
  return await rpcCall('get_stats', {});
};
