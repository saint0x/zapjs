/**
 * ZapJS CLI Logger - Orange-themed visual logging utility
 *
 * Brand Colors (from zap.svg logo):
 * - Primary Orange: #ec751a
 * - Light Orange: #f5ba77
 * - Dark Background: #0a0a0a
 */

import chalk from 'chalk';
import ora, { Ora } from 'ora';

// Define brand colors
const COLORS = {
  primary: '#ec751a',      // Main orange from logo
  light: '#f5ba77',        // Light orange for accents
  success: '#10b981',      // Green for success
  error: '#ef4444',        // Red for errors
  warning: '#f59e0b',      // Amber for warnings
  info: '#3b82f6',         // Blue for info
  dim: '#6b7280',          // Gray for secondary text
} as const;

// Custom chalk instances with brand colors
const orange = chalk.hex(COLORS.primary);
const orangeLight = chalk.hex(COLORS.light);
const successColor = chalk.hex(COLORS.success);
const errorColor = chalk.hex(COLORS.error);
const warningColor = chalk.hex(COLORS.warning);
const infoColor = chalk.hex(COLORS.info);
const dim = chalk.hex(COLORS.dim);

export class CliLogger {
  private spinners: Map<string, Ora> = new Map();

  /**
   * Print ZapJS branded header
   */
  header(text: string): void {
    console.log();
    console.log(orange.bold(`⚡ ${text}`));
    console.log(dim('─'.repeat(60)));
  }

  /**
   * Success message with checkmark
   */
  success(message: string, details?: string): void {
    console.log(successColor('✓'), chalk.white(message));
    if (details) {
      console.log(dim(`  ${details}`));
    }
  }

  /**
   * Error message with X symbol
   */
  error(message: string, err?: Error | string): void {
    console.log(errorColor('✗'), chalk.white(message));
    if (err) {
      const errorMsg = typeof err === 'string' ? err : err.message;
      console.log(dim(`  ${errorMsg}`));
    }
  }

  /**
   * Warning message with warning symbol
   */
  warn(message: string, details?: string): void {
    console.log(warningColor('⚠'), chalk.white(message));
    if (details) {
      console.log(dim(`  ${details}`));
    }
  }

  /**
   * Info message with info symbol
   */
  info(message: string, details?: string): void {
    console.log(infoColor('ℹ'), chalk.white(message));
    if (details) {
      console.log(dim(`  ${details}`));
    }
  }

  /**
   * Step indicator (numbered steps)
   */
  step(num: number, message: string): void {
    const stepNum = orange(`[${num}]`);
    console.log(`${stepNum} ${chalk.white(message)}`);
  }

  /**
   * Command suggestion (dimmed with code styling)
   */
  command(cmd: string, description?: string): void {
    console.log(`  ${orangeLight('$')} ${chalk.cyan(cmd)}`);
    if (description) {
      console.log(dim(`    ${description}`));
    }
  }

  /**
   * Start a spinner with orange color
   */
  spinner(id: string, text: string): Ora {
    const spinner = ora({
      text: chalk.white(text),
      color: 'yellow', // ora doesn't support hex, yellow is closest to orange
      spinner: 'dots',
    }).start();

    this.spinners.set(id, spinner);
    return spinner;
  }

  /**
   * Update spinner text
   */
  updateSpinner(id: string, text: string): void {
    const spinner = this.spinners.get(id);
    if (spinner) {
      spinner.text = chalk.white(text);
    }
  }

  /**
   * Stop spinner with success
   */
  succeedSpinner(id: string, text?: string): void {
    const spinner = this.spinners.get(id);
    if (spinner) {
      spinner.succeed(text ? chalk.white(text) : undefined);
      this.spinners.delete(id);
    }
  }

  /**
   * Stop spinner with error
   */
  failSpinner(id: string, text?: string): void {
    const spinner = this.spinners.get(id);
    if (spinner) {
      spinner.fail(text ? chalk.white(text) : undefined);
      this.spinners.delete(id);
    }
  }

  /**
   * Print a blank line
   */
  newline(): void {
    console.log();
  }

  /**
   * Print dimmed separator line
   */
  separator(): void {
    console.log(dim('─'.repeat(60)));
  }

  /**
   * Print key-value pair
   */
  keyValue(key: string, value: string | number): void {
    console.log(`  ${dim(key + ':')} ${chalk.white(value)}`);
  }

  /**
   * Print list item with bullet
   */
  listItem(text: string, bullet: string = '•'): void {
    console.log(`  ${orange(bullet)} ${chalk.white(text)}`);
  }

  /**
   * Print a box around text
   */
  box(title: string, content: string[]): void {
    const maxLen = Math.max(title.length, ...content.map(c => c.length));
    const width = maxLen + 4;

    console.log(orange('┌' + '─'.repeat(width) + '┐'));
    console.log(orange('│') + chalk.white.bold(` ${title.padEnd(maxLen)} `) + orange('│'));
    console.log(orange('├' + '─'.repeat(width) + '┤'));

    content.forEach(line => {
      console.log(orange('│') + chalk.white(` ${line.padEnd(maxLen)} `) + orange('│'));
    });

    console.log(orange('└' + '─'.repeat(width) + '┘'));
  }
}

/**
 * Global CLI logger instance
 */
export const cliLogger = new CliLogger();

/**
 * Convenience exports
 */
export const {
  header,
  success,
  error,
  warn,
  info,
  step,
  command,
  spinner,
  updateSpinner,
  succeedSpinner,
  failSpinner,
  newline,
  separator,
  keyValue,
  listItem,
  box,
} = cliLogger;
