// Benchmarks endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/benchmarks - Returns performance benchmark data
export const GET = async () => {
  return await rpcCall('get_benchmarks', {});
};
