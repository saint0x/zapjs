// Features endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/features - Returns all ZapJS features
export const GET = async () => {
  return await rpcCall('get_features', {});
};
