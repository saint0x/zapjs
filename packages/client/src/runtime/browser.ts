/**
 * Browser-safe exports for @zap-js/client
 *
 * This entry point contains NO Node.js imports and is safe to use in:
 * - Vite builds
 * - Webpack builds
 * - Browser environments
 * - React/Vue/Svelte applications
 *
 * For server-side functionality, use @zap-js/client/node instead.
 */

// ============================================================================
// Router Exports (Client-side)
// ============================================================================

export {
  // Provider
  RouterProvider,
  // Hooks
  useRouter,
  useParams,
  usePathname,
  useSearchParams,
  useRouteMatch,
  useIsPending,
  // Components
  Link,
  NavLink,
  Outlet,
  Redirect,
  // Types
  type Router,
  type RouteDefinition,
  type RouteMatch,
  type RouterState,
  type NavigateOptions,
  type LinkProps,
} from './router.js';

// ============================================================================
// Error Handling Exports
// ============================================================================

export {
  ErrorBoundary,
  DefaultErrorComponent,
  RouteErrorContext,
  createRouteError,
  ZapError,
  type ZapRouteError,
  type ErrorComponentProps,
  type ErrorComponent,
} from './error-boundary.js';

export {
  useRouteError,
  useIsErrorState,
  useErrorState,
} from './hooks.js';

// ============================================================================
// CSRF Protection Exports
// ============================================================================

export {
  getCsrfToken,
  useCsrfToken,
  createCsrfFetch,
  CsrfTokenInput,
  CsrfForm,
  addCsrfToFormData,
  getCsrfHeaders,
  type CsrfConfig,
  type CsrfTokenInputProps,
  type CsrfFormProps,
} from './csrf.js';

// ============================================================================
// Middleware Exports (Pure TypeScript, no Node.js deps)
// ============================================================================

export {
  composeMiddleware,
  requireAuth,
  requireRole,
  routeLogger,
  preloadData,
  type MiddlewareContext,
  type MiddlewareResult,
  type MiddlewareFunction,
  type RouteMiddleware,
} from './middleware.js';

// ============================================================================
// Utility Exports (Browser-safe)
// ============================================================================

export * from './streaming-utils.js';
export * from './websockets-utils.js';

// ============================================================================
// Type Exports (Browser-safe types only)
// ============================================================================

export type {
  Handler,
  ZapRequest,
  ZapHandlerResponse,
  RouteConfig,
  HttpMethod,
  MiddlewareConfig,
  FileRouteConfig,
  // Streaming types
  StreamChunk,
  StreamingHandler,
  AnyHandler,
  StreamStartMessage,
  StreamChunkMessage,
  StreamEndMessage,
  StreamMessage,
  // WebSocket types
  WsConnection,
  WsHandler,
  WsConnectMessage,
  WsMessageMessage,
  WsCloseMessage,
  WsSendMessage,
  WsMessage,
} from './types.js';

// Re-export type guards (pure functions, no Node.js deps)
export {
  isInvokeHandlerMessage,
  isHandlerResponseMessage,
  isErrorMessage,
  isHealthCheckMessage,
  isHealthCheckResponseMessage,
  isRpcResponseMessage,
  isRpcErrorMessage,
  isAsyncIterable,
} from './types.js';

// ============================================================================
// Namespace Exports for Convenience
// ============================================================================

/**
 * Router namespace - all routing functionality
 * Usage: import { router } from '@zap-js/client/browser'
 */
import {
  RouterProvider,
  useRouter as useRouterFn,
  useParams as useParamsFn,
  usePathname as usePathnameFn,
  useSearchParams as useSearchParamsFn,
  useRouteMatch as useRouteMatchFn,
  useIsPending as useIsPendingFn,
  Link,
  NavLink,
  Outlet,
  Redirect,
} from './router.js';

export const router = {
  RouterProvider,
  useRouter: useRouterFn,
  useParams: useParamsFn,
  usePathname: usePathnameFn,
  useSearchParams: useSearchParamsFn,
  useRouteMatch: useRouteMatchFn,
  useIsPending: useIsPendingFn,
  Link,
  NavLink,
  Outlet,
  Redirect,
} as const;

/**
 * Errors namespace - error handling and boundaries
 * Usage: import { errors } from '@zap-js/client/browser'
 */
import {
  ErrorBoundary,
  DefaultErrorComponent,
  createRouteError,
  ZapError,
} from './error-boundary.js';
import {
  useRouteError,
  useIsErrorState,
  useErrorState,
} from './hooks.js';

export const errors = {
  ErrorBoundary,
  DefaultErrorComponent,
  createRouteError,
  ZapError,
  useRouteError,
  useIsErrorState,
  useErrorState,
} as const;

/**
 * Middleware namespace - route middleware utilities
 * Usage: import { middleware } from '@zap-js/client/browser'
 */
import {
  composeMiddleware,
  requireAuth,
  requireRole,
  routeLogger,
  preloadData,
} from './middleware.js';

export const middleware = {
  compose: composeMiddleware,
  requireAuth,
  requireRole,
  logger: routeLogger,
  preloadData,
} as const;

/**
 * Types namespace - type definitions and guards
 * Usage: import { types } from '@zap-js/client/browser'
 */
import * as TypeGuards from './types.js';

export const types = {
  isInvokeHandlerMessage: TypeGuards.isInvokeHandlerMessage,
  isHandlerResponseMessage: TypeGuards.isHandlerResponseMessage,
  isErrorMessage: TypeGuards.isErrorMessage,
  isHealthCheckMessage: TypeGuards.isHealthCheckMessage,
  isHealthCheckResponseMessage: TypeGuards.isHealthCheckResponseMessage,
  isRpcResponseMessage: TypeGuards.isRpcResponseMessage,
  isRpcErrorMessage: TypeGuards.isRpcErrorMessage,
  isAsyncIterable: TypeGuards.isAsyncIterable,
} as const;

/**
 * WebSockets namespace - WebSocket utilities and helpers
 * Usage: import { websockets } from '@zap-js/client/browser'
 */
import * as WebSocketUtils from './websockets-utils.js';

export const websockets = {
  isWsMessage: WebSocketUtils.isWsMessage,
  broadcast: WebSocketUtils.broadcast,
  broadcastExcept: WebSocketUtils.broadcastExcept,
  sendJson: WebSocketUtils.sendJson,
  parseMessage: WebSocketUtils.parseMessage,
  createErrorMessage: WebSocketUtils.createErrorMessage,
  createSuccessMessage: WebSocketUtils.createSuccessMessage,
} as const;

/**
 * Streaming namespace - Streaming response utilities
 * Usage: import { streaming } from '@zap-js/client/browser'
 */
import * as StreamingUtils from './streaming-utils.js';

export const streaming = {
  isAsyncIterable: StreamingUtils.isAsyncIterable,
  createChunk: StreamingUtils.createChunk,
  createStream: StreamingUtils.createStream,
  streamJson: StreamingUtils.streamJson,
  streamSSE: StreamingUtils.streamSSE,
  mapStream: StreamingUtils.mapStream,
  filterStream: StreamingUtils.filterStream,
  batchStream: StreamingUtils.batchStream,
  delayStream: StreamingUtils.delayStream,
  fromReadableStream: StreamingUtils.fromReadableStream,
  intervalStream: StreamingUtils.intervalStream,
} as const;
