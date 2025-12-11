// Single post endpoint - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

// GET /api/posts/:id - Get a specific post by ID or slug
export const GET = async ({ params }: { params: { id: string } }) => {
  const result = await rpcCall('get_post', { id: params.id });

  // Handle not found from Rust backend
  if (result && typeof result === 'object' && 'error' in result) {
    return new Response(
      JSON.stringify(result),
      { status: 404, headers: { 'Content-Type': 'application/json' } }
    );
  }

  return result;
};
