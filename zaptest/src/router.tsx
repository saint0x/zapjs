/**
 * Simple client-side router for zaptest landing site
 *
 * Demonstrates the @zap-js/client router API in a simple SPA context.
 * For full file-based routing, use the RouterProvider with generated routes.
 */

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useMemo,
  type ReactNode,
  type MouseEvent,
} from 'react';

// ============================================================================
// Types
// ============================================================================

interface RouterState {
  pathname: string;
  hash: string;
}

interface Router {
  push(path: string): void;
  replace(path: string): void;
  back(): void;
  forward(): void;
}

interface RouterContextValue {
  state: RouterState;
  router: Router;
}

interface LinkProps extends Omit<React.AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  to: string;
  replace?: boolean;
  children: ReactNode;
}

// ============================================================================
// Context
// ============================================================================

const RouterContext = createContext<RouterContextValue | null>(null);

// ============================================================================
// Provider
// ============================================================================

interface AppRouterProviderProps {
  children: ReactNode;
}

export function AppRouterProvider({ children }: AppRouterProviderProps) {
  const [state, setState] = useState<RouterState>(() => ({
    pathname: window.location.pathname,
    hash: window.location.hash,
  }));

  // Navigate function
  const navigate = useCallback((path: string, replaceState = false) => {
    const [pathname, hash = ''] = path.split('#');

    if (replaceState) {
      window.history.replaceState({}, '', path);
    } else {
      window.history.pushState({}, '', path);
    }

    setState({ pathname: pathname || '/', hash: hash ? `#${hash}` : '' });
  }, []);

  // Router API
  const router = useMemo<Router>(
    () => ({
      push: (path) => navigate(path, false),
      replace: (path) => navigate(path, true),
      back: () => window.history.back(),
      forward: () => window.history.forward(),
    }),
    [navigate]
  );

  // Handle browser back/forward and popstate events
  useEffect(() => {
    const handlePopState = () => {
      setState({
        pathname: window.location.pathname,
        hash: window.location.hash,
      });
    };

    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  }, []);

  const value = useMemo(() => ({ state, router }), [state, router]);

  return <RouterContext.Provider value={value}>{children}</RouterContext.Provider>;
}

// ============================================================================
// Hooks
// ============================================================================

/**
 * Access the router for programmatic navigation
 */
export function useRouter(): Router {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useRouter must be used within AppRouterProvider');
  }
  return context.router;
}

/**
 * Get the current pathname
 */
export function usePathname(): string {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('usePathname must be used within AppRouterProvider');
  }
  return context.state.pathname;
}

/**
 * Get the current hash
 */
export function useHash(): string {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error('useHash must be used within AppRouterProvider');
  }
  return context.state.hash;
}

// ============================================================================
// Link Component
// ============================================================================

/**
 * Client-side navigation link
 */
export function Link({
  to,
  replace = false,
  children,
  onClick,
  ...props
}: LinkProps) {
  const router = useRouter();

  const handleClick = useCallback(
    (e: MouseEvent<HTMLAnchorElement>) => {
      onClick?.(e);

      // Don't handle if default prevented, not left click, or modifier keys
      if (
        e.defaultPrevented ||
        e.button !== 0 ||
        e.metaKey ||
        e.ctrlKey ||
        e.shiftKey ||
        e.altKey
      ) {
        return;
      }

      // Don't handle external links
      if (to.startsWith('http://') || to.startsWith('https://') || to.startsWith('//')) {
        return;
      }

      e.preventDefault();
      router[replace ? 'replace' : 'push'](to);
    },
    [router, to, replace, onClick]
  );

  return (
    <a href={to} onClick={handleClick} {...props}>
      {children}
    </a>
  );
}

// ============================================================================
// NavLink Component (with active state)
// ============================================================================

interface NavLinkProps extends LinkProps {
  activeClassName?: string;
  exact?: boolean;
}

export function NavLink({
  to,
  activeClassName = '',
  exact = false,
  className = '',
  ...props
}: NavLinkProps) {
  const pathname = usePathname();

  const isActive = exact
    ? pathname === to
    : pathname.startsWith(to) && (to === '/' ? pathname === '/' : true);

  const combinedClassName = isActive
    ? `${className} ${activeClassName}`.trim()
    : className;

  return <Link to={to} className={combinedClassName} {...props} />;
}
