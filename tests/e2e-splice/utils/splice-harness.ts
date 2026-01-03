import { spawn, ChildProcess } from 'child_process';
import { tmpdir } from 'os';
import { join } from 'path';
import { existsSync, unlinkSync } from 'fs';

export interface SpliceTestConfig {
  workerBinaryPath: string;
  spliceBinaryPath: string;
  socketDir?: string;
  maxConcurrency?: number;
  timeout?: number;
}

export class SpliceTestHarness {
  private config: SpliceTestConfig;
  private spliceProcess: ChildProcess | null = null;
  private socketPath: string;
  private running: boolean = false;

  constructor(config: SpliceTestConfig) {
    this.config = config;

    const testId = Math.random().toString(36).substring(7);
    const socketDir = config.socketDir || tmpdir();
    this.socketPath = join(socketDir, `splice-test-${testId}.sock`);
  }

  async start(): Promise<void> {
    if (this.running) {
      throw new Error('Harness already running');
    }

    console.log('[Test Harness] Starting Splice...');
    console.log('[Test Harness] Worker:', this.config.workerBinaryPath);
    console.log('[Test Harness] Socket:', this.socketPath);

    const args = [
      '--socket',
      this.socketPath,
      '--worker',
      this.config.workerBinaryPath,
      '--max-concurrency',
      (this.config.maxConcurrency || 100).toString(),
      '--timeout',
      (this.config.timeout || 30).toString(),
    ];

    this.spliceProcess = spawn(this.config.spliceBinaryPath, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
      env: {
        ...process.env,
        RUST_LOG: process.env.RUST_LOG || 'info',
      },
    });

    this.spliceProcess.stdout?.on('data', (data: Buffer) => {
      const output = data.toString().trim();
      if (output) {
        console.log(`[Splice] ${output}`);
      }
    });

    this.spliceProcess.stderr?.on('data', (data: Buffer) => {
      const output = data.toString().trim();
      if (output) {
        console.error(`[Splice] ${output}`);
      }
    });

    this.spliceProcess.on('exit', (code, signal) => {
      this.running = false;
      if (code !== 0 && code !== null) {
        console.error(`[Splice] Exited: code=${code}, signal=${signal}`);
      }
    });

    this.spliceProcess.on('error', (err) => {
      this.running = false;
      console.error('[Splice] Process error:', err);
    });

    this.running = true;

    await this.waitForSocket();
    console.log('[Test Harness] Splice ready');
  }

  async stop(): Promise<void> {
    if (!this.running || !this.spliceProcess) {
      return;
    }

    console.log('[Test Harness] Stopping Splice...');

    if (!this.spliceProcess.killed) {
      this.spliceProcess.kill('SIGTERM');
    }

    await new Promise((resolve) => setTimeout(resolve, 1000));

    if (!this.spliceProcess.killed) {
      this.spliceProcess.kill('SIGKILL');
    }

    this.spliceProcess = null;
    this.running = false;

    if (existsSync(this.socketPath)) {
      try {
        unlinkSync(this.socketPath);
      } catch {
        // Ignore cleanup errors
      }
    }

    console.log('[Test Harness] Cleanup complete');
  }

  getSocketPath(): string {
    return this.socketPath;
  }

  isRunning(): boolean {
    return this.running && this.spliceProcess !== null && !this.spliceProcess.killed;
  }

  private async waitForSocket(): Promise<void> {
    const maxWait = 10000; // 10 seconds
    const checkInterval = 100;
    const startTime = Date.now();

    while (Date.now() - startTime < maxWait) {
      if (existsSync(this.socketPath)) {
        await new Promise((resolve) => setTimeout(resolve, 500));
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, checkInterval));
    }

    throw new Error('Splice socket not ready within timeout');
  }
}

export async function invokeViaHttp(
  port: number,
  functionName: string,
  params: any,
  headers?: Record<string, string>
): Promise<any> {
  const response = await fetch(`http://127.0.0.1:${port}/api/rpc`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...headers,
    },
    body: JSON.stringify({
      function: functionName,
      params,
    }),
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }

  return response.json();
}
