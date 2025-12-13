import { execSync } from 'child_process';
import { cliLogger } from '../utils/logger.js';

export interface CodegenOptions {
  input?: string;
  output?: string;
}

/**
 * Generate TypeScript bindings from Rust exports
 */
export async function codegenCommand(options: CodegenOptions): Promise<void> {
  const outputDir = options.output || './src/api';

  try {
    cliLogger.header('Generating TypeScript Bindings');

    cliLogger.spinner('codegen', `Generating bindings to ${outputDir}...`);

    try {
      let cmd = 'zap-codegen';

      if (options.output) {
        cmd += ` --output ${options.output}`;
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
      cliLogger.error('Make sure zap-codegen is installed');
      cliLogger.command('npm install -g @zapjs/codegen');
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
