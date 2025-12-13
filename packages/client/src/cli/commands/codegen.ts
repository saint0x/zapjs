import { execSync } from 'child_process';
import { existsSync } from 'fs';
import { join } from 'path';
import { cliLogger } from '../utils/logger.js';

export interface CodegenOptions {
  input?: string;
  output?: string;
}

/**
 * Find the zap-codegen binary in known locations
 */
function findCodegenBinary(): string | null {
  const projectDir = process.cwd();

  // Check multiple possible locations
  const possiblePaths = [
    // Project bin directory
    join(projectDir, 'bin', 'zap-codegen'),
    join(projectDir, 'bin', 'zap-codegen.exe'),
    // Workspace target (for monorepo development)
    join(projectDir, '../../target/release/zap-codegen'),
    join(projectDir, '../../target/aarch64-apple-darwin/release/zap-codegen'),
    join(projectDir, '../../target/x86_64-unknown-linux-gnu/release/zap-codegen'),
    join(projectDir, '../../target/debug/zap-codegen'),
    // Local target directory
    join(projectDir, 'target/release/zap-codegen'),
    join(projectDir, 'target/debug/zap-codegen'),
  ];

  for (const path of possiblePaths) {
    if (existsSync(path)) {
      return path;
    }
  }

  // Try global zap-codegen as fallback
  try {
    execSync('which zap-codegen', { stdio: 'pipe' });
    return 'zap-codegen';
  } catch {
    // Not in PATH
  }

  return null;
}

/**
 * Generate TypeScript bindings from Rust exports
 */
export async function codegenCommand(options: CodegenOptions): Promise<void> {
  const outputDir = options.output || './src/api';

  try {
    cliLogger.header('Generating TypeScript Bindings');

    // Find the codegen binary
    const codegenBinary = findCodegenBinary();

    if (!codegenBinary) {
      cliLogger.error('zap-codegen binary not found');
      cliLogger.newline();
      cliLogger.info('Please build the codegen binary first:');
      cliLogger.command('cargo build --release --bin zap-codegen');
      cliLogger.newline();
      cliLogger.info('Or install globally:');
      cliLogger.command('npm install -g @zapjs/codegen');
      cliLogger.newline();
      process.exit(1);
    }

    cliLogger.spinner('codegen', `Generating bindings to ${outputDir}...`);

    try {
      let cmd = `"${codegenBinary}"`;

      if (options.output) {
        cmd += ` --output-dir ${options.output}`;
      } else {
        cmd += ` --output-dir ${outputDir}`;
      }

      if (options.input) {
        cmd += ` --input ${options.input}`;
      }

      execSync(cmd, {
        stdio: 'pipe',
      });

      cliLogger.succeedSpinner('codegen', 'TypeScript bindings generated');
    } catch (error) {
      cliLogger.failSpinner('codegen', 'Codegen failed');
      if (error instanceof Error) {
        cliLogger.error('Error details', error.message);
      }
      process.exit(1);
    }

    cliLogger.newline();
    cliLogger.success('Codegen complete!');
    cliLogger.keyValue('Generated files in', outputDir);
    cliLogger.newline();
  } catch (error) {
    cliLogger.error('Codegen failed');
    if (error instanceof Error) {
      cliLogger.error('Error details', error.message);
    }
    process.exit(1);
  }
}
