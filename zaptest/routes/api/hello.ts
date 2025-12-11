// Hello endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/hello - Simple hello endpoint
export const GET = async () => {
  return await rpcCall('hello', {});
};

// POST /api/hello - Echo received data
export const POST = async ({ request }: { request: Request }) => {
  try {
    const body = await request.json();
    return {
      received: body,
      message: 'Data received successfully',
      timestamp: new Date().toISOString(),
    };
  } catch {
    return {
      error: 'Invalid JSON body',
      code: 'INVALID_JSON',
    };
  }
};
