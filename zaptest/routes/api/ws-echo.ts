/**
 * WebSocket Echo Demo
 *
 * Demonstrates ZapJS WebSocket capabilities using WEBSOCKET export.
 * The WEBSOCKET export is detected by the route scanner and handled
 * by the Rust server's WebSocket module.
 *
 * Connect at: ws://localhost:3000/api/ws-echo
 */

import type { WsConnection, WsHandler } from '@zap-js/client';

// Track connected clients
const clients = new Map<string, { connection: WsConnection; connectedAt: number }>();

export const WEBSOCKET: WsHandler = {
  onConnect: async (connection: WsConnection) => {
    clients.set(connection.id, {
      connection,
      connectedAt: Date.now()
    });

    // Send welcome message
    connection.send(JSON.stringify({
      type: 'connected',
      id: connection.id,
      message: 'Welcome to ZapJS WebSocket demo!',
      totalClients: clients.size,
      timestamp: Date.now()
    }));

    // Notify other clients
    for (const [id, client] of clients) {
      if (id !== connection.id) {
        client.connection.send(JSON.stringify({
          type: 'user_joined',
          userId: connection.id,
          totalClients: clients.size,
          timestamp: Date.now()
        }));
      }
    }

    console.log(`[WS] Client connected: ${connection.id}, total: ${clients.size}`);
  },

  onMessage: async (connection: WsConnection, message: string | Uint8Array) => {
    const messageStr = typeof message === 'string' ? message : new TextDecoder().decode(message);
    console.log(`[WS] Message from ${connection.id}:`, messageStr);

    // Parse message
    let parsed: { type?: string; content?: string } = {};
    try {
      parsed = JSON.parse(messageStr);
    } catch {
      parsed = { type: 'text', content: messageStr };
    }

    // Handle message types
    switch (parsed.type) {
      case 'ping':
        connection.send(JSON.stringify({
          type: 'pong',
          timestamp: Date.now()
        }));
        break;

      case 'broadcast':
        for (const client of clients.values()) {
          client.connection.send(JSON.stringify({
            type: 'broadcast',
            from: connection.id,
            content: parsed.content,
            timestamp: Date.now()
          }));
        }
        break;

      case 'stats':
        connection.send(JSON.stringify({
          type: 'stats',
          totalClients: clients.size,
          clients: Array.from(clients.entries()).map(([id, c]) => ({
            id,
            connectedAt: c.connectedAt,
            uptime: Date.now() - c.connectedAt
          })),
          timestamp: Date.now()
        }));
        break;

      default:
        // Echo back
        connection.send(JSON.stringify({
          type: 'echo',
          original: messageStr,
          clientId: connection.id,
          timestamp: Date.now()
        }));
    }
  },

  onClose: async (connection: WsConnection, code?: number, reason?: string) => {
    const client = clients.get(connection.id);
    const uptime = client ? Date.now() - client.connectedAt : 0;

    clients.delete(connection.id);
    console.log(`[WS] Client disconnected: ${connection.id}, code: ${code}, uptime: ${uptime}ms`);

    // Notify remaining clients
    for (const c of clients.values()) {
      c.connection.send(JSON.stringify({
        type: 'user_left',
        userId: connection.id,
        totalClients: clients.size,
        timestamp: Date.now()
      }));
    }
  },

  onError: async (connection: WsConnection, error: Error) => {
    console.error(`[WS] Error for ${connection.id}:`, error.message);
    connection.send(JSON.stringify({
      type: 'error',
      message: error.message,
      timestamp: Date.now()
    }));
  }
};

// Also export a GET handler to return info about the WebSocket endpoint
export const GET = async () => {
  return {
    endpoint: '/api/ws-echo',
    protocol: 'websocket',
    description: 'WebSocket echo server demo',
    commands: {
      ping: 'Send { type: "ping" } to receive a pong',
      broadcast: 'Send { type: "broadcast", content: "..." } to broadcast to all clients',
      stats: 'Send { type: "stats" } to get connection statistics',
      echo: 'Send any other message to receive an echo'
    },
    connectedClients: clients.size
  };
};
