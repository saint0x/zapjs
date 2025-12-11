/**
 * Route file scanner for ZapJS (TanStack style conventions)
 *
 * TanStack Router Conventions:
 * - index.tsx          → /
 * - about.tsx          → /about
 * - $param.tsx         → /:param (required)
 * - $param?.tsx        → /:param? (optional)
 * - $...rest.tsx       → /*rest (required catch-all)
 * - $...rest?.tsx      → /*rest? (optional catch-all)
 * - posts.$postId.tsx  → /posts/:postId
 * - _layout.tsx        → Layout scoped to directory segment
 * - __root.tsx         → Root layout
 * - (group)/           → Route group (no URL segment)
 * - -excluded/         → Excluded from routing
 * - api/*.ts           → API routes (separate folder)
 * - ws/*.ts            → WebSocket routes (dedicated folder)
 */

import { readdirSync, statSync, existsSync, readFileSync } from 'fs';
import { join, relative, extname, basename, dirname } from 'path';
import type {
  ScannedRoute,
  LayoutRoute,
  RootRoute,
  RouteTree,
  RouteParam,
  RouteType,
  ScanOptions,
  HttpMethod,
  WebSocketRoute,
} from './types.js';

const DEFAULT_EXTENSIONS = ['.tsx', '.ts', '.jsx', '.js'];
const API_FOLDER = 'api';
const WS_FOLDER = 'ws';
const HTTP_METHODS: HttpMethod[] = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'];

/**
 * Result of detecting special exports in a route file
 */
interface RouteExports {
  hasErrorComponent: boolean;
  errorComponentExport?: string;
  hasPendingComponent: boolean;
  pendingComponentExport?: string;
  hasMeta: boolean;
  hasMiddleware: boolean;
  hasWebSocket: boolean;
  methods?: HttpMethod[];
}

/**
 * Detect special exports in a route file
 *
 * Looks for TanStack Router style exports:
 * - errorComponent - Custom error boundary component
 * - pendingComponent - Loading/pending state component
 * - meta - Route metadata function
 * - middleware - Route middleware array
 * - WEBSOCKET - WebSocket handler object
 * - HTTP method handlers (GET, POST, etc.) for API routes
 */
function detectRouteExports(filePath: string, isApiRoute: boolean): RouteExports {
  try {
    const content = readFileSync(filePath, 'utf-8');

    const result: RouteExports = {
      hasErrorComponent: false,
      hasPendingComponent: false,
      hasMeta: false,
      hasMiddleware: false,
      hasWebSocket: false,
    };

    // Detect errorComponent export
    // Patterns:
    // - export const errorComponent = ...
    // - export function errorComponent(...
    // - export { errorComponent }
    // - export { SomeComponent as errorComponent }
    const errorComponentPatterns = [
      /export\s+(?:const|let|var)\s+errorComponent\b/,
      /export\s+function\s+errorComponent\s*\(/,
      /export\s*\{\s*(?:[\w]+\s+as\s+)?errorComponent\s*[,}]/,
    ];

    for (const pattern of errorComponentPatterns) {
      if (pattern.test(content)) {
        result.hasErrorComponent = true;
        result.errorComponentExport = 'errorComponent';
        break;
      }
    }

    // Detect pendingComponent export
    const pendingComponentPatterns = [
      /export\s+(?:const|let|var)\s+pendingComponent\b/,
      /export\s+function\s+pendingComponent\s*\(/,
      /export\s*\{\s*(?:[\w]+\s+as\s+)?pendingComponent\s*[,}]/,
    ];

    for (const pattern of pendingComponentPatterns) {
      if (pattern.test(content)) {
        result.hasPendingComponent = true;
        result.pendingComponentExport = 'pendingComponent';
        break;
      }
    }

    // Detect meta export (route metadata)
    const metaPatterns = [
      /export\s+(?:const|let|var)\s+meta\b/,
      /export\s+(?:async\s+)?function\s+meta\s*\(/,
      /export\s*\{\s*(?:[\w]+\s+as\s+)?meta\s*[,}]/,
    ];

    for (const pattern of metaPatterns) {
      if (pattern.test(content)) {
        result.hasMeta = true;
        break;
      }
    }

    // Detect middleware export
    const middlewarePatterns = [
      /export\s+(?:const|let|var)\s+middleware\b/,
      /export\s*\{\s*(?:[\w]+\s+as\s+)?middleware\s*[,}]/,
    ];

    for (const pattern of middlewarePatterns) {
      if (pattern.test(content)) {
        result.hasMiddleware = true;
        break;
      }
    }

    // Detect WEBSOCKET export
    const wsPatterns = [
      /export\s+(?:const|let|var)\s+WEBSOCKET\b/,
      /export\s*\{\s*(?:[\w]+\s+as\s+)?WEBSOCKET\s*[,}]/,
    ];

    for (const pattern of wsPatterns) {
      if (pattern.test(content)) {
        result.hasWebSocket = true;
        break;
      }
    }

    // For API routes, detect HTTP method exports
    if (isApiRoute) {
      const detectedMethods: HttpMethod[] = [];
      for (const method of HTTP_METHODS) {
        // Patterns for HTTP method exports:
        // - export const GET = ...
        // - export function GET(...
        // - export async function GET(...
        // - export { GET }
        const methodPatterns = [
          new RegExp(`export\\s+(?:const|let|var)\\s+${method}\\b`),
          new RegExp(`export\\s+(?:async\\s+)?function\\s+${method}\\s*\\(`),
          new RegExp(`export\\s*\\{\\s*(?:[\\w]+\\s+as\\s+)?${method}\\s*[,}]`),
        ];

        for (const pattern of methodPatterns) {
          if (pattern.test(content)) {
            detectedMethods.push(method);
            break;
          }
        }
      }
      if (detectedMethods.length > 0) {
        result.methods = detectedMethods;
      }
    }

    return result;
  } catch {
    // If we can't read the file, return defaults
    return {
      hasErrorComponent: false,
      hasPendingComponent: false,
      hasMeta: false,
      hasMiddleware: false,
      hasWebSocket: false,
      methods: isApiRoute ? HTTP_METHODS : undefined,
    };
  }
}

/**
 * Calculate route priority score
 * Higher score = more specific route
 * Static segments > dynamic segments > catch-all
 */
function calculateRoutePriority(urlPath: string, params: RouteParam[]): number {
  let score = 0;
  const segments = urlPath.split('/').filter(Boolean);

  // Base score: 1000 points per segment (ensures longer paths generally win)
  score += segments.length * 1000;

  for (let i = 0; i < segments.length; i++) {
    const segment = segments[i];

    if (segment.startsWith('*')) {
      // Catch-all: lowest priority (0 points)
      // Optional catch-all even lower
      const isOptional = params.some(p => p.catchAll && p.optional);
      score += isOptional ? -100 : 0;
    } else if (segment.startsWith(':')) {
      // Dynamic segment: medium priority (100 points)
      const paramName = segment.slice(1).replace('?', '');
      const isOptional = params.some(p => p.name === paramName && p.optional);
      score += isOptional ? 50 : 100;
    } else {
      // Static segment: high priority (500 points)
      score += 500;
    }
  }

  // Index routes get a small bonus
  if (urlPath === '/') {
    score += 10;
  }

  return score;
}

export class RouteScanner {
  private routesDir: string;
  private extensions: string[];
  private includeApi: boolean;

  constructor(options: ScanOptions) {
    this.routesDir = options.routesDir;
    this.extensions = options.extensions ?? DEFAULT_EXTENSIONS;
    this.includeApi = options.includeApi ?? true;
  }

  /**
   * Scan the routes directory and build a route tree
   */
  scan(): RouteTree {
    if (!existsSync(this.routesDir)) {
      return {
        root: null,
        routes: [],
        layouts: [],
        apiRoutes: [],
        wsRoutes: [],
      };
    }

    const routes: ScannedRoute[] = [];
    const layouts: LayoutRoute[] = [];
    const apiRoutes: ScannedRoute[] = [];
    const wsRoutes: WebSocketRoute[] = [];
    let root: RootRoute | null = null;

    this.scanDirectory(this.routesDir, '', routes, layouts, apiRoutes, wsRoutes, (r) => {
      root = r;
    });

    // Assign layout paths to routes based on directory scope
    this.assignLayouts(routes, layouts);

    // Calculate priorities and sort by priority (descending)
    routes.sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));
    apiRoutes.sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0));

    return { root, routes, layouts, apiRoutes, wsRoutes };
  }

  /**
   * Assign parent layouts to routes based on directory structure
   */
  private assignLayouts(routes: ScannedRoute[], layouts: LayoutRoute[]): void {
    // Sort layouts by scope path length (longest first for most specific match)
    const sortedLayouts = [...layouts].sort((a, b) => b.scopePath.length - a.scopePath.length);

    for (const route of routes) {
      const routeDir = dirname(route.relativePath);

      // Find the most specific layout that contains this route
      for (const layout of sortedLayouts) {
        if (routeDir === layout.scopePath || routeDir.startsWith(layout.scopePath + '/') || layout.scopePath === '') {
          route.layoutPath = layout.filePath;
          break;
        }
      }
    }

    // Also set parent layouts for nested layouts
    for (let i = 0; i < layouts.length; i++) {
      const layout = layouts[i];

      for (const parentLayout of sortedLayouts) {
        if (parentLayout === layout) continue;

        const layoutDir = dirname(layout.relativePath);
        if (layoutDir.startsWith(parentLayout.scopePath + '/') || (parentLayout.scopePath === '' && layoutDir !== '')) {
          layout.parentLayout = parentLayout.filePath;
          break;
        }
      }
    }
  }

  private scanDirectory(
    dir: string,
    pathPrefix: string,
    routes: ScannedRoute[],
    layouts: LayoutRoute[],
    apiRoutes: ScannedRoute[],
    wsRoutes: WebSocketRoute[],
    setRoot: (root: RootRoute) => void
  ): void {
    const entries = readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = join(dir, entry.name);
      const relativePath = join(pathPrefix, entry.name);

      if (entry.isDirectory()) {
        // Skip excluded directories (prefixed with -)
        if (entry.name.startsWith('-')) {
          continue;
        }

        // Handle route groups (parentheses)
        if (entry.name.startsWith('(') && entry.name.endsWith(')')) {
          // Route group - no URL segment
          this.scanDirectory(fullPath, pathPrefix, routes, layouts, apiRoutes, wsRoutes, setRoot);
          continue;
        }

        // Handle API folder
        if (entry.name === API_FOLDER && this.includeApi) {
          this.scanApiDirectory(fullPath, '/api', apiRoutes, wsRoutes);
          continue;
        }

        // Handle WebSocket folder
        if (entry.name === WS_FOLDER) {
          this.scanWsDirectory(fullPath, '/ws', wsRoutes);
          continue;
        }

        // Regular directory - add to path
        const urlSegment = this.fileNameToUrlSegment(entry.name);
        const newPrefix = pathPrefix ? `${pathPrefix}/${entry.name}` : entry.name;
        this.scanDirectory(fullPath, newPrefix, routes, layouts, apiRoutes, wsRoutes, setRoot);
        continue;
      }

      // Handle files
      if (!this.isRouteFile(entry.name)) {
        continue;
      }

      const baseName = this.getBaseName(entry.name);

      // Handle root layout
      if (baseName === '__root') {
        const rootLayout: RootRoute = {
          type: 'root',
          filePath: fullPath,
          relativePath,
          urlPath: '/',
          children: [],
          scopePath: '',
        };
        setRoot(rootLayout);
        continue;
      }

      // Handle layouts - scoped to their directory segment
      if (baseName === '_layout') {
        const layout: LayoutRoute = {
          filePath: fullPath,
          relativePath,
          urlPath: this.prefixToUrl(pathPrefix),
          children: [],
          scopePath: pathPrefix, // Directory scope for nested layouts
        };
        layouts.push(layout);
        continue;
      }

      // Regular route
      const route = this.parseRouteFile(fullPath, relativePath, pathPrefix, baseName);
      routes.push(route);
    }
  }

  private scanApiDirectory(
    dir: string,
    urlPrefix: string,
    apiRoutes: ScannedRoute[],
    wsRoutes: WebSocketRoute[]
  ): void {
    const entries = readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = join(dir, entry.name);
      const relativePath = relative(this.routesDir, fullPath);

      if (entry.isDirectory()) {
        if (entry.name.startsWith('-')) continue;

        const urlSegment = this.fileNameToUrlSegment(entry.name);
        this.scanApiDirectory(fullPath, `${urlPrefix}/${urlSegment}`, apiRoutes, wsRoutes);
        continue;
      }

      if (!this.isRouteFile(entry.name)) {
        continue;
      }

      const baseName = this.getBaseName(entry.name);
      const route = this.parseApiRouteFile(fullPath, relativePath, urlPrefix, baseName);

      // Check if this API route has WEBSOCKET export
      const exports = detectRouteExports(fullPath, true);
      if (exports.hasWebSocket) {
        wsRoutes.push({
          filePath: fullPath,
          relativePath,
          urlPath: route.urlPath,
          params: route.params,
        });
      }

      apiRoutes.push(route);
    }
  }

  /**
   * Scan dedicated WebSocket folder
   */
  private scanWsDirectory(
    dir: string,
    urlPrefix: string,
    wsRoutes: WebSocketRoute[]
  ): void {
    const entries = readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = join(dir, entry.name);
      const relativePath = relative(this.routesDir, fullPath);

      if (entry.isDirectory()) {
        if (entry.name.startsWith('-')) continue;

        const urlSegment = this.fileNameToUrlSegment(entry.name);
        this.scanWsDirectory(fullPath, `${urlPrefix}/${urlSegment}`, wsRoutes);
        continue;
      }

      if (!this.isRouteFile(entry.name)) {
        continue;
      }

      const baseName = this.getBaseName(entry.name);
      const wsRoute = this.parseWsRouteFile(fullPath, relativePath, urlPrefix, baseName);
      wsRoutes.push(wsRoute);
    }
  }

  /**
   * Parse a WebSocket route file
   */
  private parseWsRouteFile(
    filePath: string,
    relativePath: string,
    urlPrefix: string,
    baseName: string
  ): WebSocketRoute {
    const params: RouteParam[] = [];
    let urlPath: string;

    if (baseName === 'index') {
      urlPath = urlPrefix;
    } else {
      const segments = baseName.split('.');
      const urlSegments: string[] = [];
      let paramIndex = urlPrefix.split('/').filter(Boolean).length;

      for (const segment of segments) {
        const parsed = this.parseSegment(segment, paramIndex);
        params.push(...parsed.params);
        urlSegments.push(parsed.urlSegment);
        paramIndex++;
      }

      urlPath = `${urlPrefix}/${urlSegments.join('/')}`;
    }

    return {
      filePath,
      relativePath,
      urlPath,
      params,
    };
  }

  /**
   * Parse a single segment for params (supports optional syntax: $param? and $...rest?)
   */
  private parseSegment(segment: string, paramIndex: number): { urlSegment: string; params: RouteParam[] } {
    const params: RouteParam[] = [];

    if (segment.startsWith('$')) {
      // Dynamic segment
      let paramName = segment.slice(1);
      const isCatchAll = paramName.startsWith('...');
      if (isCatchAll) {
        paramName = paramName.slice(3);
      }

      // Check for optional marker (?)
      const isOptional = paramName.endsWith('?');
      if (isOptional) {
        paramName = paramName.slice(0, -1);
      }

      params.push({
        name: paramName,
        index: paramIndex,
        catchAll: isCatchAll,
        optional: isOptional,
      });

      // URL segment format: :param or :param? for optional, *rest or *rest? for catch-all
      const urlSegment = isCatchAll
        ? `*${paramName}${isOptional ? '?' : ''}`
        : `:${paramName}${isOptional ? '?' : ''}`;

      return { urlSegment, params };
    }

    return { urlSegment: segment, params: [] };
  }

  private parseRouteFile(
    filePath: string,
    relativePath: string,
    pathPrefix: string,
    baseName: string
  ): ScannedRoute {
    const params: RouteParam[] = [];
    let urlPath: string;
    let isIndex = false;

    if (baseName === 'index') {
      // Index route
      urlPath = this.prefixToUrl(pathPrefix);
      isIndex = true;
    } else {
      // Parse the base name (may have dot-separated segments)
      const segments = baseName.split('.');
      const urlSegments: string[] = [];

      let paramIndex = pathPrefix.split('/').filter(Boolean).length;

      for (const segment of segments) {
        const parsed = this.parseSegment(segment, paramIndex);
        params.push(...parsed.params);
        urlSegments.push(parsed.urlSegment);
        paramIndex++;
      }

      const base = this.prefixToUrl(pathPrefix);
      urlPath = base === '/'
        ? `/${urlSegments.join('/')}`
        : `${base}/${urlSegments.join('/')}`;
    }

    // Detect special exports
    const exports = detectRouteExports(filePath, false);

    // Calculate priority
    const priority = calculateRoutePriority(urlPath, params);

    return {
      filePath,
      relativePath,
      urlPath,
      type: 'page',
      params,
      isIndex,
      hasErrorComponent: exports.hasErrorComponent || undefined,
      errorComponentExport: exports.errorComponentExport,
      hasPendingComponent: exports.hasPendingComponent || undefined,
      pendingComponentExport: exports.pendingComponentExport,
      hasMeta: exports.hasMeta || undefined,
      hasMiddleware: exports.hasMiddleware || undefined,
      priority,
    };
  }

  private parseApiRouteFile(
    filePath: string,
    relativePath: string,
    urlPrefix: string,
    baseName: string
  ): ScannedRoute {
    const params: RouteParam[] = [];
    let urlPath: string;
    let isIndex = false;

    if (baseName === 'index') {
      urlPath = urlPrefix;
      isIndex = true;
    } else {
      const segments = baseName.split('.');
      const urlSegments: string[] = [];
      let paramIndex = urlPrefix.split('/').filter(Boolean).length;

      for (const segment of segments) {
        const parsed = this.parseSegment(segment, paramIndex);
        params.push(...parsed.params);
        urlSegments.push(parsed.urlSegment);
        paramIndex++;
      }

      urlPath = `${urlPrefix}/${urlSegments.join('/')}`;
    }

    // Detect HTTP method exports in the API route file
    const exports = detectRouteExports(filePath, true);

    // Calculate priority
    const priority = calculateRoutePriority(urlPath, params);

    return {
      filePath,
      relativePath,
      urlPath,
      type: 'api',
      params,
      methods: exports.methods ?? HTTP_METHODS, // Fall back to all methods if detection fails
      isIndex,
      hasMiddleware: exports.hasMiddleware || undefined,
      priority,
    };
  }

  private isRouteFile(fileName: string): boolean {
    const ext = extname(fileName);
    return this.extensions.includes(ext);
  }

  private getBaseName(fileName: string): string {
    const ext = extname(fileName);
    return basename(fileName, ext);
  }

  private fileNameToUrlSegment(name: string): string {
    // Handle dynamic segments
    if (name.startsWith('$')) {
      const paramName = name.slice(1);
      if (paramName.startsWith('...')) {
        return `*${paramName.slice(3)}`;
      }
      return `:${paramName}`;
    }
    return name;
  }

  private prefixToUrl(prefix: string): string {
    if (!prefix) return '/';

    const segments = prefix.split('/').filter(Boolean);
    const urlSegments = segments.map((s) => this.fileNameToUrlSegment(s));

    return '/' + urlSegments.join('/');
  }
}

/**
 * Convenience function to scan routes
 */
export function scanRoutes(routesDir: string, options?: Partial<ScanOptions>): RouteTree {
  const scanner = new RouteScanner({
    routesDir,
    ...options,
  });
  return scanner.scan();
}

/**
 * Convert route tree to a flat list for debugging/display
 */
export function flattenRoutes(tree: RouteTree): ScannedRoute[] {
  return [...tree.routes, ...tree.apiRoutes];
}

/**
 * Get the parent layout for a route
 */
export function findParentLayout(
  route: ScannedRoute,
  layouts: LayoutRoute[]
): LayoutRoute | null {
  // Find the layout with the longest matching path prefix
  let bestMatch: LayoutRoute | null = null;
  let bestLength = -1;

  for (const layout of layouts) {
    if (route.urlPath.startsWith(layout.urlPath) && layout.urlPath.length > bestLength) {
      bestMatch = layout;
      bestLength = layout.urlPath.length;
    }
  }

  return bestMatch;
}
