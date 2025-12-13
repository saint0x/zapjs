/**
 * CSRF (Cross-Site Request Forgery) Protection Utilities
 *
 * Provides client-side utilities for working with CSRF tokens:
 * - Reading tokens from cookies
 * - Including tokens in fetch requests
 * - Form components with automatic token injection
 */

import { useState, useEffect } from 'react';

/**
 * CSRF configuration
 */
export interface CsrfConfig {
  /** Cookie name for CSRF token (default: "csrf_token") */
  cookieName?: string;
  /** Header name for CSRF token (default: "X-CSRF-Token") */
  headerName?: string;
  /** Form field name for CSRF token (default: "_csrf") */
  formFieldName?: string;
}

const DEFAULT_CONFIG: Required<CsrfConfig> = {
  cookieName: 'csrf_token',
  headerName: 'X-CSRF-Token',
  formFieldName: '_csrf',
};

/**
 * Get CSRF token from cookie
 */
export function getCsrfToken(config: CsrfConfig = {}): string | null {
  const { cookieName } = { ...DEFAULT_CONFIG, ...config };

  if (typeof document === 'undefined') {
    return null;
  }

  const cookies = document.cookie.split(';');
  for (const cookie of cookies) {
    const [name, value] = cookie.trim().split('=');
    if (name === cookieName) {
      return value;
    }
  }

  return null;
}

/**
 * React hook to get CSRF token
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const csrfToken = useCsrfToken();
 *
 *   const handleSubmit = async () => {
 *     await fetch('/api/data', {
 *       method: 'POST',
 *       headers: {
 *         'X-CSRF-Token': csrfToken || '',
 *         'Content-Type': 'application/json',
 *       },
 *       body: JSON.stringify({ data: 'value' }),
 *     });
 *   };
 * }
 * ```
 */
export function useCsrfToken(config: CsrfConfig = {}): string | null {
  const [token, setToken] = useState<string | null>(() => getCsrfToken(config));

  useEffect(() => {
    // Update token if cookie changes
    const interval = setInterval(() => {
      const newToken = getCsrfToken(config);
      if (newToken !== token) {
        setToken(newToken);
      }
    }, 1000);

    return () => clearInterval(interval);
  }, [config, token]);

  return token;
}

/**
 * Create a fetch wrapper that automatically includes CSRF token
 *
 * @example
 * ```tsx
 * const csrfFetch = createCsrfFetch();
 *
 * // Token is automatically included in POST/PUT/DELETE/PATCH requests
 * await csrfFetch('/api/data', {
 *   method: 'POST',
 *   body: JSON.stringify({ data: 'value' }),
 * });
 * ```
 */
export function createCsrfFetch(config: CsrfConfig = {}): typeof fetch {
  const { headerName } = { ...DEFAULT_CONFIG, ...config };

  return async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const token = getCsrfToken(config);

    // Only add token for state-changing methods
    const method = init?.method?.toUpperCase() || 'GET';
    const needsToken = ['POST', 'PUT', 'DELETE', 'PATCH'].includes(method);

    if (needsToken && token) {
      const headers = new Headers(init?.headers);
      if (!headers.has(headerName)) {
        headers.set(headerName, token);
      }

      return fetch(input, {
        ...init,
        headers,
      });
    }

    return fetch(input, init);
  };
}

/**
 * CSRF token input component for forms
 *
 * @example
 * ```tsx
 * function MyForm() {
 *   return (
 *     <form method="POST" action="/api/submit">
 *       <CsrfTokenInput />
 *       <input name="username" />
 *       <button type="submit">Submit</button>
 *     </form>
 *   );
 * }
 * ```
 */
export interface CsrfTokenInputProps {
  /** Configuration for CSRF token retrieval */
  config?: CsrfConfig;
}

export function CsrfTokenInput({ config = {} }: CsrfTokenInputProps) {
  const token = useCsrfToken(config);
  const { formFieldName } = { ...DEFAULT_CONFIG, ...config };

  if (!token) {
    console.warn('CSRF token not found. Form submission may fail.');
    return null;
  }

  return <input type="hidden" name={formFieldName} value={token} />;
}

/**
 * Enhanced form component with automatic CSRF protection
 *
 * @example
 * ```tsx
 * function MyPage() {
 *   const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
 *     e.preventDefault();
 *     const formData = new FormData(e.currentTarget);
 *     await fetch('/api/submit', {
 *       method: 'POST',
 *       body: formData,
 *     });
 *   };
 *
 *   return (
 *     <CsrfForm onSubmit={handleSubmit}>
 *       <input name="username" />
 *       <button type="submit">Submit</button>
 *     </CsrfForm>
 *   );
 * }
 * ```
 */
export interface CsrfFormProps extends React.FormHTMLAttributes<HTMLFormElement> {
  /** Configuration for CSRF token retrieval */
  config?: CsrfConfig;
  /** Child elements */
  children: React.ReactNode;
}

export function CsrfForm({ config, children, ...formProps }: CsrfFormProps) {
  return (
    <form {...formProps}>
      <CsrfTokenInput config={config} />
      {children}
    </form>
  );
}

/**
 * Utility to manually add CSRF token to FormData
 *
 * @example
 * ```tsx
 * const formData = new FormData();
 * formData.append('username', 'john');
 * addCsrfToFormData(formData);
 *
 * await fetch('/api/submit', {
 *   method: 'POST',
 *   body: formData,
 * });
 * ```
 */
export function addCsrfToFormData(formData: FormData, config: CsrfConfig = {}): void {
  const token = getCsrfToken(config);
  const { formFieldName } = { ...DEFAULT_CONFIG, ...config };

  if (token) {
    formData.append(formFieldName, token);
  } else {
    console.warn('CSRF token not found. FormData submission may fail.');
  }
}

/**
 * Utility to get headers object with CSRF token
 *
 * @example
 * ```tsx
 * const headers = {
 *   'Content-Type': 'application/json',
 *   ...getCsrfHeaders(),
 * };
 *
 * await fetch('/api/data', {
 *   method: 'POST',
 *   headers,
 *   body: JSON.stringify({ data: 'value' }),
 * });
 * ```
 */
export function getCsrfHeaders(config: CsrfConfig = {}): Record<string, string> {
  const token = getCsrfToken(config);
  const { headerName } = { ...DEFAULT_CONFIG, ...config };

  if (!token) {
    console.warn('CSRF token not found. Request may fail.');
    return {};
  }

  return {
    [headerName]: token,
  };
}
