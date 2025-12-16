/**
 * @zap-js/server
 *
 * ZapJS server communication utilities
 * Re-exports from @zap-js/client
 */

// Namespace exports (Clean API)

/**
 * RPC namespace - Remote Procedure Call utilities
 * Usage: import { rpc } from '@zap-js/server'
 * Then: rpc.call('functionName', args)
 */
import { rpcCall } from '@zap-js/client';
export const rpc = {
  call: rpcCall,
};

/**
 * Types namespace - re-export from client
 * Usage: import { types } from '@zap-js/server'
 */
import { types as clientTypes } from '@zap-js/client';
export const types = clientTypes;
