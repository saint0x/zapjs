/**
 * RPC Client for ZapJS Server Functions
 * Auto-generated - DO NOT EDIT MANUALLY
 */

const IPC_ENDPOINT = 'http://127.0.0.1:3000/__zap_rpc';

interface RpcRequest {
  method: string;
  params: Record<string, unknown>;
}

interface RpcResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/**
 * Make an RPC call to a Rust server function
 */
export async function rpcCall<T>(
  method: string,
  params: Record<string, unknown> = {}
): Promise<T> {
  const request: RpcRequest = { method, params };

  const response = await fetch(IPC_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`RPC call failed: ${error}`);
  }

  const result: RpcResponse<T> = await response.json();

  if (!result.success) {
    throw new Error(result.error || 'Unknown RPC error');
  }

  return result.data as T;
}

/**
 * Batch multiple RPC calls
 */
export async function rpcBatch<T extends unknown[]>(
  calls: Array<{ method: string; params: Record<string, unknown> }>
): Promise<T> {
  const results = await Promise.all(
    calls.map((call) => rpcCall(call.method, call.params))
  );
  return results as T;
}
