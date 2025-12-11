// Single user operations - calls Rust backend via RPC
import { rpcCall } from '../../src/generated/rpc-client';

interface ZapRequest {
  method: string;
  path: string;
  path_only: string;
  query: Record<string, string>;
  params: Record<string, string>;
  headers: Record<string, string>;
  body: string;
  cookies: Record<string, string>;
}

// GET /api/users/:id - Get a specific user
export const GET = async ({ params }: { params: { id: string } }) => {
  const result = await rpcCall('get_user', { id: params.id });

  if (result && typeof result === 'object' && 'error' in result) {
    return new Response(
      JSON.stringify(result),
      { status: 404, headers: { 'Content-Type': 'application/json' } }
    );
  }

  return result;
};

// PUT /api/users/:id - Update a specific user
export const PUT = async ({ params, request }: { params: { id: string }; request?: Request } & ZapRequest) => {
  try {
    let name: string | undefined;
    let email: string | undefined;
    let role: string | undefined;

    // Handle both Request object and ZapRequest body string
    if (request && typeof request.json === 'function') {
      const body = await request.json();
      name = body.name;
      email = body.email;
      role = body.role;
    }

    const result = await rpcCall('update_user', {
      id: params.id,
      name: name || null,
      email: email || null,
      role: role || null,
    });

    if (result && typeof result === 'object' && 'error' in result) {
      return new Response(
        JSON.stringify(result),
        { status: 404, headers: { 'Content-Type': 'application/json' } }
      );
    }

    return result;
  } catch {
    return new Response(
      JSON.stringify({ error: 'Invalid JSON body', code: 'INVALID_JSON' }),
      { status: 400, headers: { 'Content-Type': 'application/json' } }
    );
  }
};

// DELETE /api/users/:id - Delete a specific user
export const DELETE = async ({ params }: { params: { id: string } }) => {
  const result = await rpcCall('delete_user', { id: params.id });

  if (result && typeof result === 'object' && 'error' in result) {
    return new Response(
      JSON.stringify(result),
      { status: 404, headers: { 'Content-Type': 'application/json' } }
    );
  }

  return result;
};
