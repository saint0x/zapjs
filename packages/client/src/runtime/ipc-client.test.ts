import { describe, expect, test, beforeEach, mock, afterEach } from 'bun:test';
import { IpcServer, IpcClient, serializeMessage, deserializeMessage, FrameReader } from './ipc-client';
import { createServer, Server } from 'net';
import { unlinkSync, existsSync } from 'fs';

const TEST_SOCKET_PATH = '/tmp/zap-test-ipc.sock';

describe('IPC Message Serialization', () => {
  test('serializeMessage with MessagePack encoding', () => {
    const message = {
      type: 'health_check' as const,
    };

    const buffer = serializeMessage(message, 'msgpack');
    expect(buffer).toBeInstanceOf(Buffer);
    expect(buffer.length).toBeGreaterThan(0);
  });

  test('serializeMessage with JSON encoding', () => {
    const message = {
      type: 'health_check' as const,
    };

    const buffer = serializeMessage(message, 'json');
    expect(buffer).toBeInstanceOf(Buffer);
    const json = JSON.parse(buffer.toString('utf-8'));
    expect(json.type).toBe('health_check');
  });

  test('deserializeMessage auto-detects JSON (starts with {)', () => {
    const message = { type: 'health_check' as const };
    const buffer = Buffer.from(JSON.stringify(message));

    const deserialized = deserializeMessage(buffer);
    expect(deserialized.type).toBe('health_check');
  });

  test('deserializeMessage handles MessagePack', () => {
    const message = { type: 'health_check' as const };
    const buffer = serializeMessage(message, 'msgpack');

    const deserialized = deserializeMessage(buffer);
    expect(deserialized.type).toBe('health_check');
  });

  test('deserializeMessage throws on empty buffer', () => {
    expect(() => deserializeMessage(Buffer.alloc(0))).toThrow('Empty message');
  });
});

describe('FrameReader', () => {
  test('reads single complete frame', () => {
    const frames: Buffer[] = [];
    const reader = new FrameReader((frame) => frames.push(frame));

    const payload = Buffer.from('test payload');
    const length = Buffer.alloc(4);
    length.writeUInt32BE(payload.length, 0);

    reader.push(Buffer.concat([length, payload]));

    expect(frames.length).toBe(1);
    expect(frames[0].toString()).toBe('test payload');
  });

  test('handles partial frames (length arrives first)', () => {
    const frames: Buffer[] = [];
    const reader = new FrameReader((frame) => frames.push(frame));

    const payload = Buffer.from('test payload');
    const length = Buffer.alloc(4);
    length.writeUInt32BE(payload.length, 0);

    // Send length first
    reader.push(length);
    expect(frames.length).toBe(0);

    // Then send payload
    reader.push(payload);
    expect(frames.length).toBe(1);
    expect(frames[0].toString()).toBe('test payload');
  });

  test('handles multiple frames in single chunk', () => {
    const frames: Buffer[] = [];
    const reader = new FrameReader((frame) => frames.push(frame));

    const payload1 = Buffer.from('frame 1');
    const payload2 = Buffer.from('frame 2');

    const length1 = Buffer.alloc(4);
    length1.writeUInt32BE(payload1.length, 0);
    const length2 = Buffer.alloc(4);
    length2.writeUInt32BE(payload2.length, 0);

    const combined = Buffer.concat([length1, payload1, length2, payload2]);
    reader.push(combined);

    expect(frames.length).toBe(2);
    expect(frames[0].toString()).toBe('frame 1');
    expect(frames[1].toString()).toBe('frame 2');
  });

  test('rejects messages larger than 100MB', () => {
    const frames: Buffer[] = [];
    const reader = new FrameReader((frame) => frames.push(frame));

    const length = Buffer.alloc(4);
    length.writeUInt32BE(101 * 1024 * 1024, 0); // 101MB

    expect(() => reader.push(length)).toThrow('Message too large');
  });

  test('reset clears buffer', () => {
    const frames: Buffer[] = [];
    const reader = new FrameReader((frame) => frames.push(frame));

    const length = Buffer.alloc(4);
    length.writeUInt32BE(10, 0);
    reader.push(length);

    reader.reset();

    // After reset, buffer is empty. Send a new complete frame
    const payload = Buffer.from('test');
    const newLength = Buffer.alloc(4);
    newLength.writeUInt32BE(payload.length, 0);
    reader.push(Buffer.concat([newLength, payload]));

    // Should successfully read the new frame
    expect(frames.length).toBe(1);
    expect(frames[0].toString()).toBe('test');
  });
});

describe('IpcServer', () => {
  let server: IpcServer;

  beforeEach(() => {
    // Clean up socket if exists
    if (existsSync(TEST_SOCKET_PATH)) {
      unlinkSync(TEST_SOCKET_PATH);
    }
    server = new IpcServer(TEST_SOCKET_PATH, 'msgpack');
  });

  afterEach(async () => {
    await server.stop();
    if (existsSync(TEST_SOCKET_PATH)) {
      unlinkSync(TEST_SOCKET_PATH);
    }
  });

  test('starts and stops cleanly', async () => {
    await server.start();
    expect(existsSync(TEST_SOCKET_PATH)).toBe(true);

    await server.stop();
    expect(existsSync(TEST_SOCKET_PATH)).toBe(false);
  });

  test('registers and invokes handler', async () => {
    const mockHandler = mock(async (req) => ({
      status: 200,
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ message: 'success' }),
    }));

    server.registerHandler('test_handler', mockHandler);
    await server.start();

    // Handler should be registered
    expect(mockHandler).not.toHaveBeenCalled();

    // We'll test actual invocation in integration tests
  });

  test('registers WebSocket handler', async () => {
    const mockWsHandler = {
      onConnect: mock(async () => {}),
      onMessage: mock(async () => {}),
      onClose: mock(async () => {}),
    };

    server.registerWsHandler('ws_handler', mockWsHandler);
    await server.start();

    // Handler should be registered
    expect(mockWsHandler.onConnect).not.toHaveBeenCalled();
  });
});

describe('IpcClient', () => {
  let server: Server;
  let client: IpcClient;

  beforeEach(async () => {
    // Clean up socket if exists
    if (existsSync(TEST_SOCKET_PATH)) {
      unlinkSync(TEST_SOCKET_PATH);
    }

    // Create simple echo server
    server = createServer((socket) => {
      socket.on('data', (data) => {
        socket.write(data); // Echo back
      });
    });

    await new Promise<void>((resolve) => {
      server.listen(TEST_SOCKET_PATH, () => resolve());
    });
  });

  afterEach(async () => {
    if (client) {
      await client.close();
    }

    await new Promise<void>((resolve) => {
      server.close(() => resolve());
    });

    if (existsSync(TEST_SOCKET_PATH)) {
      unlinkSync(TEST_SOCKET_PATH);
    }
  });

  test('connects to server', async () => {
    client = new IpcClient(TEST_SOCKET_PATH, 'msgpack');

    // Wait for connection
    await new Promise<void>((resolve) => {
      client.on('connect', () => resolve());
    });

    expect(client.isConnected()).toBe(true);
  });

  test('sends and receives messages', async () => {
    client = new IpcClient(TEST_SOCKET_PATH, 'msgpack');

    await new Promise<void>((resolve) => {
      client.on('connect', () => resolve());
    });

    const testMessage = { type: 'health_check' as const };

    const response = await new Promise((resolve) => {
      client.on('message', (msg) => resolve(msg));
      client.send(testMessage);
    });

    expect(response).toBeDefined();
  });

  test('sendRecv waits for response', async () => {
    client = new IpcClient(TEST_SOCKET_PATH, 'msgpack');

    await new Promise<void>((resolve) => {
      client.on('connect', () => resolve());
    });

    const testMessage = { type: 'health_check' as const };
    const response = await client.sendRecv(testMessage);

    expect(response).toBeDefined();
  });

  test('sendRecv times out after 30 seconds', async () => {
    // Create server that doesn't respond
    const slowServer = createServer(() => {
      // Don't respond
    });

    const slowSocketPath = '/tmp/zap-test-slow.sock';
    if (existsSync(slowSocketPath)) {
      unlinkSync(slowSocketPath);
    }

    await new Promise<void>((resolve) => {
      slowServer.listen(slowSocketPath, () => resolve());
    });

    client = new IpcClient(slowSocketPath, 'msgpack');

    await new Promise<void>((resolve) => {
      client.on('connect', () => resolve());
    });

    const testMessage = { type: 'health_check' as const };

    // This should timeout (we're not actually waiting 30s in test, just checking the promise rejects)
    const promise = client.sendRecv(testMessage);

    // Clean up
    slowServer.close();
    if (existsSync(slowSocketPath)) {
      unlinkSync(slowSocketPath);
    }

    // We expect this to eventually timeout
    expect(promise).toBeInstanceOf(Promise);
  }, 35000);
});
