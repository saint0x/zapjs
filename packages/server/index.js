/**
 * @zap-js/server
 *
 * ZapJS server communication utilities
 * Re-exports from @zap-js/client
 */

// Re-export individual items for backward compatibility
export {
  IpcClient,
  IpcServer,
  ProcessManager,
  rpcCall,
} from '@zap-js/client';

// Namespace exports (Clean API)

/**
 * RPC namespace - Remote Procedure Call utilities
 * Usage: import { rpc } from '@zap-js/server'
 * Then: rpc.call()
 */
import { rpcCall } from '@zap-js/client';
export const rpc = {
  call: rpcCall,
};

/**
 * IPC namespace - Inter-Process Communication
 * Usage: import { ipc } from '@zap-js/server'
 */
import { IpcClient, IpcServer, ProcessManager } from '@zap-js/client';
export const ipc = {
  IpcClient,
  IpcServer,
  ProcessManager,
};

/**
 * Types namespace - re-export from client
 * Usage: import { types } from '@zap-js/server'
 */
import { types as clientTypes } from '@zap-js/client';
export const types = clientTypes;
