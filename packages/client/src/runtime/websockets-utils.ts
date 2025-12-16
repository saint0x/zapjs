/**
 * WebSocket utilities for ZapJS
 * Helper functions for working with WebSocket connections
 */

import type { WsConnection, WsMessage } from './types.js';

/**
 * Type guard to check if a message is a WebSocket message
 */
export function isWsMessage(msg: any): msg is WsMessage {
  return (
    msg &&
    typeof msg === 'object' &&
    'type' in msg &&
    (msg.type === 'ws_connect' ||
      msg.type === 'ws_message' ||
      msg.type === 'ws_close' ||
      msg.type === 'ws_send')
  );
}

/**
 * Broadcast a message to multiple WebSocket connections
 * @param connections - Array of WebSocket connections
 * @param data - Data to send (string or object that will be JSON stringified)
 * @param options - Broadcast options
 */
export function broadcast(
  connections: WsConnection[],
  data: string | Record<string, any>,
  options?: {
    exclude?: string[]; // Connection IDs to exclude
    binary?: boolean;
  }
): void {
  const message = typeof data === 'string' ? data : JSON.stringify(data);
  const excludeSet = new Set(options?.exclude || []);

  for (const connection of connections) {
    if (!excludeSet.has(connection.id)) {
      if (options?.binary) {
        const encoder = new TextEncoder();
        connection.sendBinary(encoder.encode(message));
      } else {
        connection.send(message);
      }
    }
  }
}

/**
 * Broadcast to all connections except the sender
 * @param connections - Array of all connections
 * @param senderId - ID of the sender connection to exclude
 * @param data - Data to broadcast
 */
export function broadcastExcept(
  connections: WsConnection[],
  senderId: string,
  data: string | Record<string, any>
): void {
  broadcast(connections, data, { exclude: [senderId] });
}

/**
 * Send a JSON message to a WebSocket connection
 * @param connection - WebSocket connection
 * @param data - Object to send as JSON
 */
export function sendJson(connection: WsConnection, data: Record<string, any>): void {
  connection.send(JSON.stringify(data));
}

/**
 * Parse incoming WebSocket message data
 * @param message - Message data (string or Uint8Array)
 * @returns Parsed data (string or parsed JSON if valid)
 */
export function parseMessage(message: string | Uint8Array): string | any {
  const text = typeof message === 'string' ? message : new TextDecoder().decode(message);

  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

/**
 * Create a WebSocket error response
 * @param error - Error message or Error object
 * @returns JSON string with error format
 */
export function createErrorMessage(error: string | Error): string {
  return JSON.stringify({
    type: 'error',
    message: error instanceof Error ? error.message : error,
    timestamp: Date.now(),
  });
}

/**
 * Create a WebSocket success response
 * @param data - Response data
 * @returns JSON string with success format
 */
export function createSuccessMessage(data: any): string {
  return JSON.stringify({
    type: 'success',
    data,
    timestamp: Date.now(),
  });
}
