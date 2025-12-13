import { existsSync, mkdirSync, readFileSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { cliLogger } from '../utils/logger.js';

// ESM equivalent of __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export interface RoutesOptions {
  routesDir?: string;
  output?: string;
  json?: boolean;
  showCode?: boolean;
  verbose?: boolean;
}

/**
 * Extract handler code from a route file
 */
function extractHandlerCode(filePath: string, method?: string): string | null {
  try {
    const content = readFileSync(filePath, 'utf-8');
    
    if (method) {
      // Look for specific HTTP method handler
      const patterns = [
        // export const GET = ...
        new RegExp(`export\\s+(?:const|let|var)\\s+${method}\\s*=\\s*([^;]+)`, 's'),
        // export function GET() { ... }
        new RegExp(`export\\s+(?:async\\s+)?function\\s+${method}\\s*\\([^)]*\\)\\s*{([^}]+)}`, 's'),
        // export async function GET() { ... }
        new RegExp(`export\\s+async\\s+function\\s+${method}\\s*\\([^)]*\\)\\s*{([^}]+)}`, 's'),
      ];
      
      for (const pattern of patterns) {
        const match = content.match(pattern);
        if (match) {
          return match[0].trim();
        }
      }
    } else {
      // Look for default export (page component)
      const patterns = [
        // export default function Component() { ... }
        /export\s+default\s+(?:async\s+)?function\s+\w*\s*\([^)]*\)\s*{[^}]+}/s,
        // const Component = () => { ... }; export default Component;
        /(?:const|let|var)\s+(\w+)\s*=\s*(?:\([^)]*\)|[^=]+)\s*=>\s*{[^}]+}.*export\s+default\s+\1/s,
        // export default () => { ... }
        /export\s+default\s+(?:\([^)]*\)|[^=]+)\s*=>\s*{[^}]+}/s,
      ];
      
      for (const pattern of patterns) {
        const match = content.match(pattern);
        if (match) {
          // Limit to first 10 lines for preview
          const lines = match[0].split('\n').slice(0, 10);
          if (lines.length >= 10) {
            lines.push('  // ...');
          }
          return lines.join('\n');
        }
      }
    }
    
    return null;
  } catch {
    return null;
  }
}

/**
 * Enhanced route scanner that shows handler logic
 */
export async function routesCommand(options: RoutesOptions): Promise<void> {
  try {
    const projectDir = process.cwd();
    const routesDir = resolve(options.routesDir || join(projectDir, 'routes'));
    const outputDir = resolve(options.output || join(projectDir, 'src', 'generated'));
    const showCode = options.showCode !== false; // Default to true

    cliLogger.header('ZapJS Route Scanner');

    // Check if routes directory exists
    if (!existsSync(routesDir)) {
      cliLogger.warn('No routes directory found');
      cliLogger.keyValue('Expected', routesDir);
      cliLogger.newline();
      cliLogger.info('Create a routes/ directory with your route files to get started');
      cliLogger.newline();
      cliLogger.info('Next.js-style conventions:');
      cliLogger.listItem('routes/index.tsx          → /');
      cliLogger.listItem('routes/about.tsx          → /about');
      cliLogger.listItem('routes/[postId].tsx       → /:postId');
      cliLogger.listItem('routes/posts/[id].tsx     → /posts/:id');
      cliLogger.listItem('routes/api/users.ts       → /api/users');
      cliLogger.listItem('routes/_layout.tsx        → Layout wrapper');
      cliLogger.listItem('routes/__root.tsx         → Root layout');
      cliLogger.newline();
      return;
    }

    // Try to load the router package
    cliLogger.spinner('loader', 'Loading route scanner...');

    let router: any;

    try {
      // Path from dist/cli/commands/routes.js to dist/router/index.js
      const routerPath = join(__dirname, '../../router/index.js');

      if (existsSync(routerPath)) {
        router = await import(routerPath);
      } else {
        throw new Error(`Router module not found at ${routerPath}`);
      }
    } catch (error) {
      cliLogger.failSpinner('loader', 'Route scanner not found');
      cliLogger.error('Error', error instanceof Error ? error.message : String(error));
      return;
    }

    cliLogger.succeedSpinner('loader', 'Route scanner loaded');

    // Scan routes
    cliLogger.spinner('scan', `Scanning ${routesDir}...`);
    const tree = router.scanRoutes(routesDir);
    cliLogger.succeedSpinner('scan', 'Routes scanned');

    // Output JSON if requested
    if (options.json) {
      console.log(JSON.stringify(tree, null, 2));
      return;
    }

    // Print route summary with code
    cliLogger.newline();
    cliLogger.info('📁 Page Routes:');
    cliLogger.newline();
    if (tree.routes.length === 0) {
      console.log('    (none)');
    } else {
      for (const route of tree.routes) {
        const params = route.params.length > 0
          ? ` [${route.params.map((p: { name: string }) => p.name).join(', ')}]`
          : '';
        const index = route.isIndex ? ' (index)' : '';

        console.log(`  ${route.urlPath}${params}${index}`);
        console.log(`    File: ${route.relativePath}`);

        if (showCode) {
          const code = extractHandlerCode(route.filePath);
          if (code && options.verbose) {
            console.log('    Handler:');
            const codeLines = code.split('\n').map(line => '      ' + line);
            console.log(codeLines.join('\n'));
          }
        }

        // Show special exports
        const features = [];
        if (route.hasErrorComponent) features.push('error boundary');
        if (route.hasPendingComponent) features.push('loading state');
        if (route.hasMeta) features.push('meta tags');
        if (route.hasMiddleware) features.push('middleware');
        if (route.hasGenerateStaticParams) features.push('SSG');

        if (features.length > 0) {
          console.log(`    Features: ${features.join(', ')}`);
        }

        console.log();
      }
    }

    cliLogger.newline();
    cliLogger.info('🌐 API Routes:');
    cliLogger.newline();
    if (tree.apiRoutes.length === 0) {
      console.log('    (none)');
    } else {
      for (const route of tree.apiRoutes) {
        const params = route.params.length > 0
          ? ` [${route.params.map((p: { name: string }) => p.name).join(', ')}]`
          : '';
        const methods = route.methods
          ? ` ${route.methods.join(' | ')}`
          : '';

        console.log(`  ${route.urlPath}${params}`);
        console.log(`    File: ${route.relativePath}`);
        console.log(`    Methods:${methods}`);

        if (showCode && route.methods) {
          for (const method of route.methods) {
            const code = extractHandlerCode(route.filePath, method);
            if (code) {
              if (options.verbose) {
                console.log(`    ${method} Handler:`);
                const codeLines = code.split('\n').map(line => '      ' + line);
                console.log(codeLines.join('\n'));
              } else {
                // Just show first line
                const firstLine = code.split('\n')[0];
                console.log(`    ${method}: ${firstLine.trim()}...`);
              }
            }
          }
        }

        // Show features
        const features = [];
        if (route.hasMiddleware) features.push('middleware');

        if (features.length > 0) {
          console.log(`    Features: ${features.join(', ')}`);
        }

        console.log();
      }
    }

    // Show layouts
    if (tree.layouts && tree.layouts.length > 0) {
      cliLogger.newline();
      cliLogger.info('📐 Layouts:');
      cliLogger.newline();
      for (const layout of tree.layouts) {
        console.log(`  ${layout.scopePath || '/'} (scope)`);
        console.log(`    File: ${layout.relativePath}`);
        if (layout.parentLayout) {
          console.log(`    Parent: ${layout.parentLayout}`);
        }
        console.log();
      }
    }

    // Show WebSocket routes
    if (tree.wsRoutes && tree.wsRoutes.length > 0) {
      cliLogger.newline();
      cliLogger.info('🔌 WebSocket Routes:');
      cliLogger.newline();
      for (const route of tree.wsRoutes) {
        const params = route.params.length > 0
          ? ` [${route.params.map((p: { name: string }) => p.name).join(', ')}]`
          : '';

        console.log(`  ${route.urlPath}${params}`);
        console.log(`    File: ${route.relativePath}`);
        console.log();
      }
    }

    // Generate route tree files if output is specified
    if (!options.json) {
      cliLogger.spinner('generate', 'Generating route tree...');

      if (!existsSync(outputDir)) {
        mkdirSync(outputDir, { recursive: true });
      }

      // Use enhanced route tree generation if available
      if (router.generateEnhancedRouteTree) {
        router.generateEnhancedRouteTree({
          outputDir,
          routeTree: tree,
        });
      } else {
        router.generateRouteTree({
          outputDir,
          routeTree: tree,
        });
      }

      cliLogger.succeedSpinner('generate', 'Route tree generated');

      // Summary
      const totalRoutes = tree.routes.length + tree.apiRoutes.length + (tree.wsRoutes?.length || 0);
      cliLogger.newline();
      cliLogger.success(`Found ${totalRoutes} total routes:`);
      console.log(`  - ${tree.routes.length} page routes`);
      console.log(`  - ${tree.apiRoutes.length} API routes`);
      if (tree.wsRoutes?.length) {
        console.log(`  - ${tree.wsRoutes.length} WebSocket routes`);
      }
      if (tree.layouts?.length) {
        console.log(`  - ${tree.layouts.length} layouts`);
      }
      cliLogger.newline();
      cliLogger.keyValue('Output', outputDir);
      cliLogger.newline();
    }

    // Tips
    if (!options.verbose && showCode) {
      cliLogger.info('💡 Tip: Use --verbose flag to see full handler code');
      cliLogger.newline();
    }

  } catch (error) {
    cliLogger.error('Route scanning failed');
    if (error instanceof Error) {
      cliLogger.error('Error details', error.message);
      if (options.verbose) {
        console.error(error.stack);
      }
    }
    process.exit(1);
  }
}