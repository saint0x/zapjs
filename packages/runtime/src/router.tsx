/**
 * ZapJS Client Router
 *
 * A lightweight, high-performance client-side router for React.
 * Provides familiar APIs: useRouter, useParams, usePathname, useSearchParams, Link
 *
 * Usage:
 * ```tsx
 * import { RouterProvider, useRouter, useParams, Link } from '@zapjs/runtime';
 *
 * // Wrap your app
 * <RouterProvider routes={routeDefinitions}>
 *   <App />
 * </RouterProvider>
 *
 * // In components
 * const router = useRouter();
 * const params = useParams<{ id: string }>();
 * <Link to="/posts/123">View Post</Link>
 * ```
 */

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useMemo,
  useTransition,
  Suspense,
  type ReactNode,
  type ComponentType,
  type MouseEvent,
} from 'react';

// ============================================================================
// Types
// ============================================================================

export interface RouteDefinition {
  path: string;
  pattern: RegExp;
  paramNames: string[];
  component: React.LazyExoticComponent<ComponentType<any>>;
  isIndex: boolean;
  layoutPath?: string;
  errorComponent?: React.LazyExoticComponent<ComponentType<any>>;
  pendingComponent?: React.LazyExoticComponent<ComponentType<any>>;
}

export interface RouteMatch {
  route: RouteDefinition;
  params: Record<string, string>;
  pathname: string;
}

export interface RouterState {
  pathname: string;
  search: string;
  hash: string;
  match: RouteMatch | null;
}

export interface NavigateOptions {
  replace?: boolean;
  scroll?: boolean;
  state?: unknown;
}

export interface Router {
  /** Navigate to a path */
  push(path: string, options?: NavigateOptions): void;
  /** Replace current history entry */
  replace(path: string, options?: NavigateOptions): void;
  /** Go back in history */
  back(): void;
  /** Go forward in history */
  forward(): void;
  /** Refresh current route */
  refresh(): void;
  /** Prefetch a route (load component) */
  prefetch(path: string): void;
}

export interface LinkProps extends Omit<React.AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  to: string;
  replace?: boolean;
  prefetch?: boolean;
  scroll?: boolean;
  children: ReactNode;
}

// ============================================================================
// Context
// ============================================================================

interface RouterContextValue {
  state: RouterState;
  router: Router;
  routes: RouteDefinition[];
  isPending: boolean;
}

const RouterContext = createContext<RouterContextValue | null>(null);

// ============================================================================
// Route Matching
// ============================================================================

/**
 * Match a pathname against route definitions
 * Returns the first matching route and extracted params
 */
function matchRoute(pathname: string, routes: RouteDefinition[]): RouteMatch | null {
  // Normalize pathname
  const normalizedPath = pathname === '' ? '/' : pathname;

  for (const route of routes) {
    const match = normalizedPath.match(route.pattern);
    if (match) {
      // Extract params from capture groups
      const params: Record<string, string> = {};
      route.paramNames.forEach((name, index) => {
        const value = match[index + 1];
        if (value !== undefined && value !== '') {
          params[name] = decodeURIComponent(value);
        }
      });

      return { route, params, pathname: normalizedPath };
    }
  }

  return null;
}

/**
 * Parse URL into components
 */
function parseUrl(url: string): { pathname: string; search: string; hash: string } {
  try {
    // Handle relative URLs
    const parsed = new URL(url, window.location.origin);
    return {
      pathname: parsed.pathname,
      search: parsed.search,
      hash: parsed.hash,
    };
  } catch {
    // Fallback for malformed URLs
    const hashIndex = url.indexOf('#');
    const searchIndex = url.indexOf('?');

    let pathname = url;
    let search = '';
    let hash = '';

    if (hashIndex !== -1) {
      hash = url.slice(hashIndex);
      pathname = url.slice(0, hashIndex);
    }

    if (searchIndex !== -1 && (hashIndex === -1 || searchIndex < hashIndex)) {
      search = pathname.slice(searchIndex, hashIndex !== -1 ? hashIndex - searchIndex : undefined);
      pathname = pathname.slice(0, searchIndex);
    }

    return { pathname: pathname || '/', search, hash };
  }
}

// ============================================================================
// RouterProvider
// ============================================================================

interface RouterProviderProps {
  routes: RouteDefinition[];
  children: ReactNode;
  /** Fallback component for when no route matches (404) */
  notFound?: ComponentType;
  /** Loading component shown during route transitions */
  fallback?: ReactNode;
}

export function RouterProvider({
  routes,
  children,
  notFound: NotFound,
  fallback = null,
}: RouterProviderProps): JSX.Element {
  const [isPending, startTransition] = useTransition();

  // Initialize state from current URL
  const [state, setState] = useState<RouterState>(() => {
    const { pathname, search, hash } = parseUrl(window.location.href);
    return {
      pathname,
      search,
      hash,
      match: matchRoute(pathname, routes),
    };
  });

  // Navigate function
  const navigate = useCallback(
    (path: string, options: NavigateOptions = {}) => {
      const { replace = false, scroll = true } = options;
      const { pathname, search, hash } = parseUrl(path);

      // Update history
      const url = pathname + search + hash;
      if (replace) {
        window.history.replaceState(options.state ?? null, '', url);
      } else {
        window.history.pushState(options.state ?? null, '', url);
      }

      // Update state with transition for Suspense
      startTransition(() => {
        setState({
          pathname,
          search,
          hash,
          match: matchRoute(pathname, routes),
        });
      });

      // Scroll to top or hash
      if (scroll) {
        if (hash) {
          const element = document.querySelector(hash);
          element?.scrollIntoView();
        } else {
          window.scrollTo(0, 0);
        }
      }
    },
    [routes]
  );

  // Router API
  const router = useMemo<Router>(
    () => ({
      push: (path, options) => navigate(path, options),
      replace: (path, options) => navigate(path, { ...options, replace: true }),
      back: () => window.history.back(),
      forward: () => window.history.forward(),
      refresh: () => {
        startTransition(() => {
          setState((prev) => ({ ...prev, match: matchRoute(prev.pathname, routes) }));
        });
      },
      prefetch: (path) => {
        const { pathname } = parseUrl(path);
        const match = matchRoute(pathname, routes);
        if (match) {
          // Trigger lazy load by importing the component
          // This preloads the chunk via dynamic import
          const component = match.route.component as any;
          if (component._payload && component._init) {
            // React lazy internal structure - trigger the load
            try {
              component._init(component._payload);
            } catch {
              // Ignore - component will load when rendered
            }
          }
        }
      },
    }),
    [navigate, routes]
  );

  // Handle browser back/forward
  useEffect(() => {
    const handlePopState = () => {
      const { pathname, search, hash } = parseUrl(window.location.href);
      startTransition(() => {
        setState({
          pathname,
          search,
          hash,
          match: matchRoute(pathname, routes),
        });
      });
    };

    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  }, [routes]);

  // Context value
  const contextValue = useMemo<RouterContextValue>(
    () => ({
      state,
      router,
      routes,
      isPending,
    }),
    [state, router, routes, isPending]
  );

  return (
    <RouterContext.Provider value={contextValue}>
      <Suspense fallback={fallback}>
        {children}
      </Suspense>
    </RouterContext.Provider>
  );
}

// ============================================================================
// Hooks
// ============================================================================

/**
 * Access the router for programmatic navigation
 *
 * ```tsx
 * const router = useRouter();
 * router.push('/dashboard');
 * router.replace('/login');
 * router.back();
 * ```
 */
export function useRouter(): Router {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useRouter must be used within a RouterProvider');
  }
  return context.router;
}

/**
 * Get route params for the current route
 *
 * ```tsx
 * // Route: /posts/[id]
 * const { id } = useParams<{ id: string }>();
 * ```
 */
export function useParams<T extends Record<string, string> = Record<string, string>>(): T {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useParams must be used within a RouterProvider');
  }
  return (context.state.match?.params ?? {}) as T;
}

/**
 * Get the current pathname
 *
 * ```tsx
 * const pathname = usePathname(); // '/posts/123'
 * ```
 */
export function usePathname(): string {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('usePathname must be used within a RouterProvider');
  }
  return context.state.pathname;
}

/**
 * Get and set search params (query string)
 *
 * ```tsx
 * const [searchParams, setSearchParams] = useSearchParams();
 * const page = searchParams.get('page');
 * setSearchParams({ page: '2' });
 * ```
 */
export function useSearchParams(): [URLSearchParams, (params: Record<string, string>) => void] {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useSearchParams must be used within a RouterProvider');
  }

  const searchParams = useMemo(
    () => new URLSearchParams(context.state.search),
    [context.state.search]
  );

  const setSearchParams = useCallback(
    (params: Record<string, string>) => {
      const newParams = new URLSearchParams(params);
      const newSearch = newParams.toString();
      const path = context.state.pathname + (newSearch ? `?${newSearch}` : '') + context.state.hash;
      context.router.push(path, { scroll: false });
    },
    [context.router, context.state.pathname, context.state.hash]
  );

  return [searchParams, setSearchParams];
}

/**
 * Get the current route match
 *
 * ```tsx
 * const match = useRouteMatch();
 * if (match) {
 *   console.log(match.route.path, match.params);
 * }
 * ```
 */
export function useRouteMatch(): RouteMatch | null {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useRouteMatch must be used within a RouterProvider');
  }
  return context.state.match;
}

/**
 * Check if a route transition is pending
 *
 * ```tsx
 * const isPending = useIsPending();
 * // Show loading indicator during navigation
 * ```
 */
export function useIsPending(): boolean {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useIsPending must be used within a RouterProvider');
  }
  return context.isPending;
}

// ============================================================================
// Link Component
// ============================================================================

/**
 * Client-side navigation link
 *
 * ```tsx
 * <Link to="/posts/123">View Post</Link>
 * <Link to="/login" replace>Login</Link>
 * ```
 */
export function Link({
  to,
  replace = false,
  prefetch = true,
  scroll = true,
  children,
  onClick,
  onMouseEnter,
  ...props
}: LinkProps): JSX.Element {
  const context = useContext(RouterContext);

  // Handle click
  const handleClick = useCallback(
    (e: MouseEvent<HTMLAnchorElement>) => {
      // Call user's onClick first
      onClick?.(e);

      // Check if we should handle this click
      if (
        e.defaultPrevented || // Already handled
        e.button !== 0 || // Not left click
        e.metaKey || // Cmd+click (Mac)
        e.ctrlKey || // Ctrl+click
        e.shiftKey || // Shift+click
        e.altKey // Alt+click
      ) {
        return;
      }

      // Check for external links
      const href = to;
      if (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('//')) {
        return; // Let browser handle external links
      }

      // Prevent default and navigate
      e.preventDefault();
      context?.router[replace ? 'replace' : 'push'](to, { scroll });
    },
    [context?.router, to, replace, scroll, onClick]
  );

  // Prefetch on hover
  const handleMouseEnter = useCallback(
    (e: MouseEvent<HTMLAnchorElement>) => {
      onMouseEnter?.(e);
      if (prefetch && context) {
        context.router.prefetch(to);
      }
    },
    [context, to, prefetch, onMouseEnter]
  );

  return (
    <a
      href={to}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      {...props}
    >
      {children}
    </a>
  );
}

// ============================================================================
// Route Outlet
// ============================================================================

interface OutletProps {
  /** Fallback component when no route matches */
  notFound?: ComponentType;
  /** Loading fallback during lazy load */
  fallback?: ReactNode;
}

/**
 * Renders the matched route component
 *
 * ```tsx
 * function App() {
 *   return (
 *     <div>
 *       <nav>...</nav>
 *       <Outlet notFound={NotFoundPage} />
 *     </div>
 *   );
 * }
 * ```
 */
export function Outlet({ notFound: NotFound, fallback = null }: OutletProps): JSX.Element | null {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('Outlet must be used within a RouterProvider');
  }

  const { match } = context.state;

  // No match - show 404
  if (!match) {
    return NotFound ? <NotFound /> : null;
  }

  const { route, params } = match;
  const Component = route.component;

  // Wrap with error boundary if route has one
  if (route.errorComponent) {
    const ErrorComponent = route.errorComponent;
    return (
      <RouteErrorBoundary fallback={<ErrorComponent />}>
        <Suspense fallback={route.pendingComponent ? <route.pendingComponent /> : fallback}>
          <Component params={params} />
        </Suspense>
      </RouteErrorBoundary>
    );
  }

  return (
    <Suspense fallback={route.pendingComponent ? <route.pendingComponent /> : fallback}>
      <Component params={params} />
    </Suspense>
  );
}

// ============================================================================
// Error Boundary for Routes
// ============================================================================

interface RouteErrorBoundaryProps {
  children: ReactNode;
  fallback: ReactNode;
}

interface RouteErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class RouteErrorBoundary extends React.Component<RouteErrorBoundaryProps, RouteErrorBoundaryState> {
  constructor(props: RouteErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): RouteErrorBoundaryState {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback;
    }
    return this.props.children;
  }
}

// ============================================================================
// NavLink Component (active state)
// ============================================================================

interface NavLinkProps extends LinkProps {
  /** Class name when link is active */
  activeClassName?: string;
  /** Style when link is active */
  activeStyle?: React.CSSProperties;
  /** Match exact path only */
  exact?: boolean;
}

/**
 * Link with active state styling
 *
 * ```tsx
 * <NavLink to="/dashboard" activeClassName="active">
 *   Dashboard
 * </NavLink>
 * ```
 */
export function NavLink({
  to,
  activeClassName,
  activeStyle,
  exact = false,
  className,
  style,
  ...props
}: NavLinkProps): JSX.Element {
  const pathname = usePathname();

  const isActive = exact
    ? pathname === to
    : pathname.startsWith(to) && (to === '/' ? pathname === '/' : true);

  const combinedClassName = isActive && activeClassName
    ? `${className || ''} ${activeClassName}`.trim()
    : className;

  const combinedStyle = isActive && activeStyle
    ? { ...style, ...activeStyle }
    : style;

  return (
    <Link
      to={to}
      className={combinedClassName}
      style={combinedStyle}
      {...props}
    />
  );
}

// ============================================================================
// Redirect Component
// ============================================================================

interface RedirectProps {
  to: string;
  replace?: boolean;
}

/**
 * Redirect to another route on render
 *
 * ```tsx
 * if (!isLoggedIn) {
 *   return <Redirect to="/login" />;
 * }
 * ```
 */
export function Redirect({ to, replace = true }: RedirectProps): null {
  const router = useRouter();

  useEffect(() => {
    router[replace ? 'replace' : 'push'](to);
  }, [router, to, replace]);

  return null;
}
