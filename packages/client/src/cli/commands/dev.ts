import path from 'path';
import { DevServer, DevServerConfig } from '../../dev-server/index.js';
import { cliLogger } from '../utils/logger.js';
import { detectBinaries, getPlatformIdentifier } from '../utils/binary-resolver.js';

export interface DevOptions {
  port?: string;
  vitePort?: string;
  open?: boolean;
  logLevel?: string;
  release?: boolean;
  skipBuild?: boolean;
  binaryPath?: string;
  codegenBinaryPath?: string;
}

/**
 * Start development server with hot reload
 *
 * Orchestrates:
 * - Rust backend compilation with file watching
 * - Vite frontend dev server
 * - Automatic TypeScript binding generation
 * - Hot reload signaling
 */
export async function devCommand(options: DevOptions): Promise<void> {
  const projectDir = process.cwd();

  // Auto-detect pre-built binaries if not explicitly provided
  const detectedBinaries = detectBinaries(projectDir);

  const config: DevServerConfig = {
    projectDir,
    rustPort: options.port ? parseInt(options.port, 10) : 3000,
    vitePort: options.vitePort ? parseInt(options.vitePort, 10) : 5173,
    logLevel: (options.logLevel as DevServerConfig['logLevel']) || 'info',
    release: options.release || false,
    skipInitialBuild: options.skipBuild || false,
    openBrowser: options.open !== false,
    binaryPath: options.binaryPath || detectedBinaries.binaryPath,
    codegenBinaryPath: options.codegenBinaryPath || detectedBinaries.codegenBinaryPath,
  };

  // Log if using pre-built binaries
  if (config.binaryPath) {
    const platformId = getPlatformIdentifier();
    cliLogger.info(`Using pre-built binary for ${platformId}`, config.binaryPath);
  }

  const server = new DevServer(config);

  // Handle graceful shutdown
  const shutdown = async () => {
    await server.stop();
    process.exit(0);
  };

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);

  try {
    await server.start();

    // Keep the process running
    await new Promise(() => {});
  } catch (error) {
    if (error instanceof Error) {
      cliLogger.error('Development server failed', error);
    }
    process.exit(1);
  }
}
