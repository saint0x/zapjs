import { existsSync, readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const SERVER_ONLY_IMPORTS = [
  '@zap-js/server',
  '@zap-js/client/node',
  '@zap-js/client/server',
];

// Path classification tiers
const ALWAYS_ALLOWED = ['/routes/api/', '/routes/ws/'];
const BUSINESS_LOGIC_ALLOWED = ['/src/api/', '/src/services/', '/src/generated/', '/src/lib/api/'];
const UI_LAYER_BLOCKED = ['/src/components/', '/src/pages/', '/src/ui/'];
const ALLOWED_FILE_PATTERNS = [/\/rpc-client\.(ts|js)$/, /\/api-client\.(ts|js)$/];

/**
 * Validates that frontend code doesn't import server-only packages
 * Returns array of error messages (empty if valid)
 */
export function validateNoServerImportsInFrontend(srcDir: string): string[] {
  const errors: string[] = [];

  function scanFile(filePath: string) {
    // Only scan TypeScript/JavaScript files
    if (!filePath.match(/\.(tsx?|jsx?)$/)) return;

    // Skip node_modules
    if (filePath.includes('node_modules')) return;

    // Tier 1: Check server-side paths first (early exit)
    for (const allowed of ALWAYS_ALLOWED) {
      if (filePath.includes(allowed)) return;
    }

    // Tier 2: Check business logic paths
    for (const allowed of BUSINESS_LOGIC_ALLOWED) {
      if (filePath.includes(allowed)) return;
    }

    // Tier 3: Check special file patterns
    for (const pattern of ALLOWED_FILE_PATTERNS) {
      if (pattern.test(filePath)) return;
    }

    // Now scan file for server imports
    try {
      const content = readFileSync(filePath, 'utf-8');
      let serverImportFound: string | null = null;

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
            serverImportFound = serverImport;
            break;
          }
        }

        if (serverImportFound) break; // Only report once per file
      }

      // Tier 4: Classify and report errors
      if (serverImportFound) {
        // Check if in UI layer (explicit block)
        let inUILayer = false;
        for (const blocked of UI_LAYER_BLOCKED) {
          if (filePath.includes(blocked)) {
            inUILayer = true;
            break;
          }
        }

        if (inUILayer) {
          errors.push(`${filePath}: Server import '${serverImportFound}' in UI layer`);
        } else {
          errors.push(`${filePath}: Server import '${serverImportFound}' in unclassified path`);
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
