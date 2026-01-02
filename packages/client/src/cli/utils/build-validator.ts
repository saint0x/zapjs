import { existsSync, readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const SERVER_ONLY_IMPORTS = [
  '@zap-js/server',
  '@zap-js/client/node',
  '@zap-js/client/server',
];

/**
 * Validates that frontend code doesn't import server-only packages
 * Returns array of error messages (empty if valid)
 */
export function validateNoServerImportsInFrontend(srcDir: string): string[] {
  const errors: string[] = [];

  function scanFile(filePath: string) {
    // Only scan TypeScript/JavaScript files
    if (!filePath.match(/\.(tsx?|jsx?)$/)) return;

    // Skip API routes (these are server-side)
    if (filePath.includes('/routes/api/')) return;
    if (filePath.includes('/routes/ws/')) return;

    // Skip node_modules
    if (filePath.includes('node_modules')) return;

    try {
      const content = readFileSync(filePath, 'utf-8');

      for (const serverImport of SERVER_ONLY_IMPORTS) {
        // Check for both single and double quotes
        const patterns = [
          `from '${serverImport}'`,
          `from "${serverImport}"`,
          `require('${serverImport}')`,
          `require("${serverImport}")`,
        ];

        for (const pattern of patterns) {
          if (content.includes(pattern)) {
            errors.push(
              `${filePath}: Illegal server import '${serverImport}' in frontend code`
            );
            break; // Only report once per file
          }
        }
      }
    } catch (err) {
      // Ignore files that can't be read
    }
  }

  function scanDir(dir: string) {
    if (!existsSync(dir)) return;

    try {
      const entries = readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const fullPath = join(dir, entry.name);

        // Skip node_modules and hidden directories
        if (entry.name === 'node_modules' || entry.name.startsWith('.')) {
          continue;
        }

        if (entry.isDirectory()) {
          scanDir(fullPath);
        } else {
          scanFile(fullPath);
        }
      }
    } catch (err) {
      // Ignore directories that can't be read
    }
  }

  scanDir(srcDir);
  return errors;
}

/**
 * Validates the entire project structure for build
 */
export function validateBuildStructure(projectDir: string): {
  valid: boolean;
  errors: string[];
  warnings: string[];
} {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Check src directory for server imports
  const srcDir = join(projectDir, 'src');
  if (existsSync(srcDir)) {
    const srcErrors = validateNoServerImportsInFrontend(srcDir);
    errors.push(...srcErrors);
  }

  // Check routes directory for server imports (excluding api and ws routes)
  const routesDir = join(projectDir, 'routes');
  if (existsSync(routesDir)) {
    const routesErrors = validateNoServerImportsInFrontend(routesDir);
    errors.push(...routesErrors);
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}
