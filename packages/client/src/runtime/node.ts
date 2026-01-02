/**
 * Node.js exports for @zap-js/client
 *
 * This entry point includes server-side functionality that requires Node.js:
 * - Zap class (server instance)
 * - ProcessManager (manages Rust binary)
 * - IpcServer/IpcClient (inter-process communication)
 * - Logger (server-side logging)
 * - RPC client utilities
 *
 * For browser/client-side functionality, use @zap-js/client/browser instead.
 */

// ============================================================================
// Server-Side Core Exports
// ============================================================================

export { Zap } from './index.js';
export { ProcessManager } from './process-manager.js';
export { IpcServer, IpcClient } from './ipc-client.js';
export { rpcCall } from './rpc-client.js';
export { Logger, logger, type LogContext, type LogLevel, type ChildLogger } from './logger.js';

// ============================================================================
// Type Exports (All server-side types)
// ============================================================================

export type {
  Handler,
  ZapRequest,
  ZapHandlerResponse,
  ZapConfig,
  RouteConfig,
  MiddlewareConfig,
  StaticFileConfig,
  StaticFileOptions,
  FileRouteConfig,
  ZapOptions,
  IpcMessage,
  InvokeHandlerMessage,
  HandlerResponseMessage,
  ErrorMessage,
  HealthCheckMessage,
  HealthCheckResponseMessage,
  HttpMethod,
  InternalHandlerFunction,
  RpcMessage,
  RpcCallMessage,
  RpcResponseMessage,
  RpcErrorMessage,
  PendingRequest,
  // Security & Observability config types
  SecurityConfig,
  SecurityHeadersConfig,
  HstsConfig,
  RateLimitConfig,
  CorsConfig,
  ObservabilityConfig,
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

// Re-export type guards
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
// Middleware Exports
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
// Re-export Browser-Safe Functionality
// ============================================================================
// Note: Server code might also need router, errors, etc. for SSR

export {
  // Router
  RouterProvider,
  useRouter,
  useParams,
  usePathname,
  useSearchParams,
  useRouteMatch,
  useIsPending,
  Link,
  NavLink,
  Outlet,
  Redirect,
  type Router,
  type RouteDefinition,
  type RouteMatch,
  type RouterState,
  type NavigateOptions,
  type LinkProps,
  // Error handling
  ErrorBoundary,
  DefaultErrorComponent,
  RouteErrorContext,
  createRouteError,
  ZapError,
  type ZapRouteError,
  type ErrorComponentProps,
  type ErrorComponent,
  useRouteError,
  useIsErrorState,
  useErrorState,
  // CSRF
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
  // Utilities
  router,
  errors,
  middleware,
  types,
  websockets,
  streaming,
} from './index.js';

// ============================================================================
// Default Export
// ============================================================================

export { default } from './index.js';
