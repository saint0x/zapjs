// SSG info endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/ssg-info - Returns SSG (Static Site Generation) metadata
export const GET = async () => {
  return await rpcCall('get_ssg_info', {});
};
