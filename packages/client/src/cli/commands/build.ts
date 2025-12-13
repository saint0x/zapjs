import { execSync } from 'child_process';
import { join, resolve } from 'path';
import { existsSync, mkdirSync, copyFileSync, readdirSync, statSync, rmSync, writeFileSync } from 'fs';
import { cliLogger } from '../utils/logger.js';

export interface BuildOptions {
  release?: boolean;
  output?: string;
  target?: string;
  skipFrontend?: boolean;
  skipCodegen?: boolean;
}

interface BuildManifest {
  version: string;
  buildTime: string;
  rustBinary: string;
  staticDir: string | null;
  env: string;
}

/**
 * Build for production
 */
export async function buildCommand(options: BuildOptions): Promise<void> {
  const outputDir = resolve(options.output || './dist');
  const startTime = Date.now();

  try {
    cliLogger.header('ZapJS Production Build');

    // Clean output directory
    if (existsSync(outputDir)) {
      cliLogger.spinner('clean', 'Cleaning output directory...');
      rmSync(outputDir, { recursive: true, force: true });
      cliLogger.succeedSpinner('clean', 'Output directory cleaned');
    }

    // Create output structure
    mkdirSync(outputDir, { recursive: true });
    mkdirSync(join(outputDir, 'bin'), { recursive: true });

    // Step 1: Build Rust binary
    await buildRust(outputDir, options);

    // Step 2: Build frontend (if not skipped)
    let staticDir: string | null = null;
    if (!options.skipFrontend) {
      staticDir = await buildFrontend(outputDir);
    }

    // Step 3: Generate TypeScript bindings
    if (!options.skipCodegen) {
      await runCodegen();
    }

    // Step 4: Create production config
    await createProductionConfig(outputDir, staticDir);

    // Step 5: Create build manifest
    const manifest = createBuildManifest(outputDir, staticDir);
    writeFileSync(
      join(outputDir, 'manifest.json'),
      JSON.stringify(manifest, null, 2)
    );

    // Summary
    const elapsed = ((Date.now() - startTime) / 1000).toFixed(2);
    const binSize = getBinarySize(join(outputDir, 'bin', 'zap'));

    cliLogger.newline();
    cliLogger.success(`Build complete in ${elapsed}s`);
    cliLogger.newline();
    cliLogger.keyValue('Directory', outputDir);
    cliLogger.keyValue('Binary', binSize);
    if (staticDir) {
      cliLogger.keyValue('Static', join(outputDir, 'static'));
    }
    cliLogger.newline();
    cliLogger.command(`cd ${outputDir} && ./bin/zap`, 'Run in production');
    cliLogger.newline();

  } catch (error) {
    if (error instanceof Error) {
      cliLogger.error('Build failed', error);
    }
    process.exit(1);
  }
}

async function buildRust(
  outputDir: string,
  options: BuildOptions
): Promise<void> {
  cliLogger.spinner('rust', 'Building Rust backend (release mode)...');

  const args = ['build', '--release', '--bin', 'zap'];

  if (options.target) {
    args.push('--target', options.target);
  }

  try {
    execSync(`cargo ${args.join(' ')}`, {
      cwd: process.cwd(),
      stdio: 'pipe',
    });

    const targetDir = options.target
      ? join('target', options.target, 'release')
      : join('target', 'release');

    const srcBinary = join(process.cwd(), targetDir, 'zap');
    const destBinary = join(outputDir, 'bin', 'zap');

    if (existsSync(srcBinary)) {
      copyFileSync(srcBinary, destBinary);
      execSync(`chmod +x "${destBinary}"`, { stdio: 'pipe' });
    } else {
      throw new Error(`Binary not found at ${srcBinary}`);
    }

    cliLogger.succeedSpinner('rust', 'Rust backend built (release + LTO)');
  } catch (error) {
    cliLogger.failSpinner('rust', 'Rust build failed');
    throw error;
  }
}

async function buildFrontend(
  outputDir: string
): Promise<string | null> {
  const viteConfig = ['vite.config.ts', 'vite.config.js', 'vite.config.mjs']
    .find(f => existsSync(join(process.cwd(), f)));

  if (!viteConfig) {
    cliLogger.info('No Vite config found, skipping frontend build');
    return null;
  }

  cliLogger.spinner('vite', 'Building frontend (Vite)...');

  try {
    execSync('npx vite build', {
      cwd: process.cwd(),
      stdio: 'pipe',
    });

    const viteDist = join(process.cwd(), 'dist');
    const staticDir = join(outputDir, 'static');

    if (existsSync(viteDist)) {
      copyDirectory(viteDist, staticDir);
      cliLogger.succeedSpinner('vite', 'Frontend built and bundled');
      return staticDir;
    } else {
      cliLogger.warn('Vite build completed but no output found');
      return null;
    }
  } catch {
    cliLogger.warn('Frontend build failed (continuing without frontend)');
    return null;
  }
}

async function runCodegen(): Promise<void> {
  cliLogger.spinner('codegen', 'Generating TypeScript bindings...');

  const codegenPaths = [
    join(process.cwd(), 'target/release/zap-codegen'),
    'zap-codegen',
  ];

  for (const codegenPath of codegenPaths) {
    try {
      execSync(`${codegenPath} --output ./src/api`, {
        cwd: process.cwd(),
        stdio: 'pipe',
      });
      cliLogger.succeedSpinner('codegen', 'TypeScript bindings generated');
      return;
    } catch {
      continue;
    }
  }

  cliLogger.info('Codegen skipped (binary not found)');
}

async function createProductionConfig(
  outputDir: string,
  staticDir: string | null
): Promise<void> {
  const config = {
    server: {
      host: '0.0.0.0',
      port: 3000,
    },
    static: staticDir ? {
      prefix: '/',
      directory: './static',
    } : null,
    logging: {
      level: 'info',
      format: 'json',
    },
  };

  writeFileSync(
    join(outputDir, 'config.json'),
    JSON.stringify(config, null, 2)
  );
}

function createBuildManifest(
  outputDir: string,
  staticDir: string | null
): BuildManifest {
  return {
    version: '1.0.0',
    buildTime: new Date().toISOString(),
    rustBinary: './bin/zap',
    staticDir: staticDir ? './static' : null,
    env: 'production',
  };
}

function copyDirectory(src: string, dest: string): void {
  mkdirSync(dest, { recursive: true });
  const entries = readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = join(src, entry.name);
    const destPath = join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDirectory(srcPath, destPath);
    } else {
      copyFileSync(srcPath, destPath);
    }
  }
}

function getBinarySize(path: string): string {
  try {
    const stats = statSync(path);
    const bytes = stats.size;
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  } catch {
    return 'unknown';
  }
}
