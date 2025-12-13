import { describe, expect, test, beforeEach } from 'bun:test';
import { render, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';
import {
  RouterProvider,
  useRouter,
  useParams,
  usePathname,
  useSearchParams,
  Link,
  NavLink,
  Outlet,
  type RouteDefinition,
} from './router';

// Test components
function TestPage({ params }: { params?: Record<string, string> }) {
  return <div data-testid="test-page">Test Page {params?.id}</div>;
}

function AboutPage() {
  return <div data-testid="about-page">About Page</div>;
}

function NotFoundPage() {
  return <div data-testid="not-found">404 Not Found</div>;
}

// Test hook consumer components
function NavigationTest() {
  const router = useRouter();
  return (
    <div>
      <button data-testid="push-btn" onClick={() => router.push('/about')}>Push</button>
      <button data-testid="back-btn" onClick={() => router.back()}>Back</button>
    </div>
  );
}

function ParamsTest() {
  const params = useParams<{ id: string }>();
  return <div data-testid="params">{params.id}</div>;
}

function PathnameTest() {
  const pathname = usePathname();
  return <div data-testid="pathname">{pathname}</div>;
}

function SearchParamsTest() {
  const [searchParams, setSearchParams] = useSearchParams();
  return (
    <div>
      <div data-testid="search-value">{searchParams.get('q')}</div>
      <button data-testid="set-search" onClick={() => setSearchParams({ q: 'test' })}>
        Set Search
      </button>
    </div>
  );
}

// Sample route definitions
const testRoutes: RouteDefinition[] = [
  {
    path: '/',
    pattern: /^\/$/,
    paramNames: [],
    component: React.lazy(() => Promise.resolve({ default: TestPage })),
  },
  {
    path: '/about',
    pattern: /^\/about$/,
    paramNames: [],
    component: React.lazy(() => Promise.resolve({ default: AboutPage })),
  },
  {
    path: '/users/:id',
    pattern: /^\/users\/([^/]+)$/,
    paramNames: ['id'],
    component: React.lazy(() => Promise.resolve({ default: TestPage })),
  },
];

describe('RouterProvider', () => {
  beforeEach(() => {
    window.history.pushState({}, '', '/');
  });

  test('renders initial route', async () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const testPage = container.querySelector('[data-testid="test-page"]');
      expect(testPage).toBeTruthy();
      expect(testPage?.textContent).toContain('Test Page');
    });
  });

  test('renders 404 when no route matches', async () => {
    window.history.pushState({}, '', '/nonexistent');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Outlet notFound={NotFoundPage} />
      </RouterProvider>
    );

    await waitFor(() => {
      const notFound = container.querySelector('[data-testid="not-found"]');
      expect(notFound).toBeTruthy();
    });
  });

  test('navigates to different route', async () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <NavigationTest />
        <Outlet />
      </RouterProvider>
    );

    const pushBtn = container.querySelector('[data-testid="push-btn"]') as HTMLElement;
    expect(pushBtn).toBeTruthy();
    fireEvent.click(pushBtn);

    await waitFor(() => {
      expect(window.location.pathname).toBe('/about');
    });
  });
});

describe('useParams hook', () => {
  beforeEach(() => {
    window.history.pushState({}, '', '/users/123');
  });

  test('extracts route parameters', async () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <ParamsTest />
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const params = container.querySelector('[data-testid="params"]');
      expect(params).toBeTruthy();
      expect(params?.textContent).toBe('123');
    });
  });

  test('decodes URL-encoded parameters', async () => {
    window.history.pushState({}, '', '/users/hello%20world');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <ParamsTest />
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const params = container.querySelector('[data-testid="params"]');
      expect(params).toBeTruthy();
      expect(params?.textContent).toBe('hello world');
    });
  });
});

describe('usePathname hook', () => {
  test('returns current pathname', async () => {
    window.history.pushState({}, '', '/about');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <PathnameTest />
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const pathname = container.querySelector('[data-testid="pathname"]');
      expect(pathname).toBeTruthy();
      expect(pathname?.textContent).toBe('/about');
    });
  });

  test('updates when pathname changes', async () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <PathnameTest />
        <NavigationTest />
        <Outlet />
      </RouterProvider>
    );

    const pushBtn = container.querySelector('[data-testid="push-btn"]') as HTMLElement;
    fireEvent.click(pushBtn);

    await waitFor(() => {
      const pathname = container.querySelector('[data-testid="pathname"]');
      expect(pathname?.textContent).toBe('/about');
    });
  });
});

describe('useSearchParams hook', () => {
  test('reads search parameters', async () => {
    window.history.pushState({}, '', '/?q=hello');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <SearchParamsTest />
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const searchValue = container.querySelector('[data-testid="search-value"]');
      expect(searchValue?.textContent).toBe('hello');
    });
  });

  test('updates search parameters', async () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <SearchParamsTest />
        <Outlet />
      </RouterProvider>
    );

    const setBtn = container.querySelector('[data-testid="set-search"]') as HTMLElement;
    fireEvent.click(setBtn);

    await waitFor(() => {
      const searchValue = container.querySelector('[data-testid="search-value"]');
      expect(searchValue?.textContent).toBe('test');
      expect(window.location.search).toBe('?q=test');
    });
  });
});

describe('Link component', () => {
  test('renders as anchor tag', () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Link to="/about">About</Link>
      </RouterProvider>
    );

    const link = container.querySelector('a');
    expect(link).toBeTruthy();
    expect(link?.getAttribute('href')).toBe('/about');
    expect(link?.textContent).toBe('About');
  });

  test('navigates on click', async () => {
    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Link to="/about">About</Link>
        <Outlet />
      </RouterProvider>
    );

    const link = container.querySelector('a') as HTMLElement;
    fireEvent.click(link);

    await waitFor(() => {
      expect(window.location.pathname).toBe('/about');
    });
  });

  test('does not navigate for external links', () => {
    const originalPathname = window.location.pathname;

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Link to="https://example.com">External</Link>
      </RouterProvider>
    );

    const link = container.querySelector('a') as HTMLElement;
    fireEvent.click(link);

    // Should not have changed pathname (external links don't navigate via router)
    expect(window.location.pathname).toBe(originalPathname);
  });
});

describe('NavLink component', () => {
  test('applies active className when route matches', async () => {
    window.history.pushState({}, '', '/about');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <NavLink to="/about" activeClassName="active">About</NavLink>
      </RouterProvider>
    );

    await waitFor(() => {
      const link = container.querySelector('a');
      expect(link?.className).toContain('active');
    });
  });

  test('does not apply active className when route does not match', () => {
    window.history.pushState({}, '', '/');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <NavLink to="/about" activeClassName="active">About</NavLink>
      </RouterProvider>
    );

    const link = container.querySelector('a');
    expect(link?.className).not.toContain('active');
  });
});

describe('Route matching', () => {
  test('matches exact routes', async () => {
    window.history.pushState({}, '', '/about');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const aboutPage = container.querySelector('[data-testid="about-page"]');
      expect(aboutPage).toBeTruthy();
    });
  });

  test('matches dynamic routes with parameters', async () => {
    window.history.pushState({}, '', '/users/42');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const page = container.querySelector('[data-testid="test-page"]');
      expect(page).toBeTruthy();
      expect(page?.textContent).toContain('42');
    });
  });

  test('normalizes empty path to /', async () => {
    window.history.pushState({}, '', '');

    const { container } = render(
      <RouterProvider routes={testRoutes}>
        <Outlet />
      </RouterProvider>
    );

    await waitFor(() => {
      const testPage = container.querySelector('[data-testid="test-page"]');
      expect(testPage).toBeTruthy();
    });
  });
});
