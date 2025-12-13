import { describe, expect, test, beforeEach } from 'bun:test';
import { render, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';
import {
  getCsrfToken,
  useCsrfToken,
  createCsrfFetch,
  CsrfTokenInput,
  CsrfForm,
  addCsrfToFormData,
  getCsrfHeaders,
} from './csrf';

// Helper to set cookies
function setCookie(name: string, value: string) {
  document.cookie = `${name}=${value}; path=/`;
}

// Helper to clear cookies
function clearCookies() {
  const cookies = document.cookie.split(';');
  for (const cookie of cookies) {
    const eqPos = cookie.indexOf('=');
    const name = eqPos > -1 ? cookie.slice(0, eqPos).trim() : cookie.trim();
    document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/`;
  }
}

describe('getCsrfToken', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('returns null when no CSRF cookie exists', () => {
    const token = getCsrfToken();
    expect(token).toBeNull();
  });

  test('returns token from default cookie name', () => {
    setCookie('csrf_token', 'test_token_123');
    const token = getCsrfToken();
    expect(token).toBe('test_token_123');
  });

  test('returns token from custom cookie name', () => {
    setCookie('custom_csrf', 'custom_token_456');
    const token = getCsrfToken({ cookieName: 'custom_csrf' });
    expect(token).toBe('custom_token_456');
  });

  test('handles cookies with spaces', () => {
    document.cookie = 'csrf_token=token123; path=/';
    const token = getCsrfToken();
    expect(token).toBe('token123');
  });
});

describe('useCsrfToken hook', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('returns null when no token exists', () => {
    function TestComponent() {
      const token = useCsrfToken();
      return <div data-testid="token">{token || 'no-token'}</div>;
    }

    const { container } = render(<TestComponent />);
    const element = container.querySelector('[data-testid="token"]');
    expect(element?.textContent).toBe('no-token');
  });

  test('returns token when cookie exists', () => {
    setCookie('csrf_token', 'hook_token_789');

    function TestComponent() {
      const token = useCsrfToken();
      return <div data-testid="token">{token || 'no-token'}</div>;
    }

    const { container } = render(<TestComponent />);
    const element = container.querySelector('[data-testid="token"]');
    expect(element?.textContent).toBe('hook_token_789');
  });
});

describe('createCsrfFetch', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('adds CSRF token to POST requests', async () => {
    setCookie('csrf_token', 'fetch_token_abc');

    const csrfFetch = createCsrfFetch();
    let capturedHeaders: Headers | undefined;

    // Mock fetch
    global.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedHeaders = new Headers(init?.headers);
      return new Response('{}', { status: 200 });
    };

    await csrfFetch('/api/data', {
      method: 'POST',
      body: JSON.stringify({ test: 'data' }),
    });

    expect(capturedHeaders?.get('X-CSRF-Token')).toBe('fetch_token_abc');
  });

  test('adds CSRF token to PUT requests', async () => {
    setCookie('csrf_token', 'put_token_def');

    const csrfFetch = createCsrfFetch();
    let capturedHeaders: Headers | undefined;

    global.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedHeaders = new Headers(init?.headers);
      return new Response('{}', { status: 200 });
    };

    await csrfFetch('/api/data', {
      method: 'PUT',
      body: JSON.stringify({ test: 'data' }),
    });

    expect(capturedHeaders?.get('X-CSRF-Token')).toBe('put_token_def');
  });

  test('does not add token to GET requests', async () => {
    setCookie('csrf_token', 'get_token_ghi');

    const csrfFetch = createCsrfFetch();
    let capturedHeaders: Headers | undefined;

    global.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedHeaders = new Headers(init?.headers);
      return new Response('{}', { status: 200 });
    };

    await csrfFetch('/api/data', { method: 'GET' });

    expect(capturedHeaders?.has('X-CSRF-Token')).toBe(false);
  });

  test('respects custom header name', async () => {
    setCookie('csrf_token', 'custom_header_token');

    const csrfFetch = createCsrfFetch({ headerName: 'X-Custom-CSRF' });
    let capturedHeaders: Headers | undefined;

    global.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedHeaders = new Headers(init?.headers);
      return new Response('{}', { status: 200 });
    };

    await csrfFetch('/api/data', {
      method: 'POST',
      body: JSON.stringify({ test: 'data' }),
    });

    expect(capturedHeaders?.get('X-Custom-CSRF')).toBe('custom_header_token');
    expect(capturedHeaders?.has('X-CSRF-Token')).toBe(false);
  });

  test('does not override existing CSRF header', async () => {
    setCookie('csrf_token', 'auto_token');

    const csrfFetch = createCsrfFetch();
    let capturedHeaders: Headers | undefined;

    global.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedHeaders = new Headers(init?.headers);
      return new Response('{}', { status: 200 });
    };

    await csrfFetch('/api/data', {
      method: 'POST',
      headers: {
        'X-CSRF-Token': 'manual_token',
      },
    });

    expect(capturedHeaders?.get('X-CSRF-Token')).toBe('manual_token');
  });
});

describe('CsrfTokenInput component', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('renders hidden input with token', () => {
    setCookie('csrf_token', 'input_token_jkl');

    const { container } = render(<CsrfTokenInput />);
    const input = container.querySelector('input[type="hidden"][name="_csrf"]') as HTMLInputElement;

    expect(input).toBeTruthy();
    expect(input?.value).toBe('input_token_jkl');
  });

  test('renders null when no token exists', () => {
    const { container } = render(<CsrfTokenInput />);
    const input = container.querySelector('input');

    expect(input).toBeNull();
  });

  test('uses custom form field name', () => {
    setCookie('csrf_token', 'custom_field_token');

    const { container } = render(<CsrfTokenInput config={{ formFieldName: 'custom_csrf_field' }} />);
    const input = container.querySelector('input[name="custom_csrf_field"]') as HTMLInputElement;

    expect(input).toBeTruthy();
    expect(input?.value).toBe('custom_field_token');
  });
});

describe('CsrfForm component', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('renders form with CSRF token input', () => {
    setCookie('csrf_token', 'form_token_mno');

    const { container } = render(
      <CsrfForm>
        <input name="username" />
        <button type="submit">Submit</button>
      </CsrfForm>
    );

    const form = container.querySelector('form');
    const csrfInput = container.querySelector('input[type="hidden"][name="_csrf"]') as HTMLInputElement;
    const usernameInput = container.querySelector('input[name="username"]');
    const button = container.querySelector('button[type="submit"]');

    expect(form).toBeTruthy();
    expect(csrfInput).toBeTruthy();
    expect(csrfInput?.value).toBe('form_token_mno');
    expect(usernameInput).toBeTruthy();
    expect(button).toBeTruthy();
  });

  test('passes form props correctly', () => {
    setCookie('csrf_token', 'props_token');

    const { container } = render(
      <CsrfForm method="POST" action="/api/submit" className="test-form">
        <input name="test" />
      </CsrfForm>
    );

    const form = container.querySelector('form');
    expect(form?.getAttribute('method')).toBe('POST');
    expect(form?.getAttribute('action')).toBe('/api/submit');
    expect(form?.getAttribute('class')).toBe('test-form');
  });
});

describe('addCsrfToFormData', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('adds token to FormData with default field name', () => {
    setCookie('csrf_token', 'formdata_token_pqr');

    const formData = new FormData();
    formData.append('username', 'john');
    addCsrfToFormData(formData);

    expect(formData.get('username')).toBe('john');
    expect(formData.get('_csrf')).toBe('formdata_token_pqr');
  });

  test('uses custom form field name', () => {
    setCookie('custom_csrf', 'custom_formdata_token');

    const formData = new FormData();
    addCsrfToFormData(formData, {
      cookieName: 'custom_csrf',
      formFieldName: 'csrf_field',
    });

    expect(formData.get('csrf_field')).toBe('custom_formdata_token');
  });

  test('does nothing when no token exists', () => {
    const formData = new FormData();
    formData.append('test', 'value');
    addCsrfToFormData(formData);

    expect(formData.get('test')).toBe('value');
    expect(formData.get('_csrf')).toBeNull();
  });
});

describe('getCsrfHeaders', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('returns headers object with token', () => {
    setCookie('csrf_token', 'headers_token_stu');

    const headers = getCsrfHeaders();

    expect(headers['X-CSRF-Token']).toBe('headers_token_stu');
  });

  test('uses custom header name', () => {
    setCookie('csrf_token', 'custom_header_name_token');

    const headers = getCsrfHeaders({ headerName: 'X-Custom-Token' });

    expect(headers['X-Custom-Token']).toBe('custom_header_name_token');
    expect(headers['X-CSRF-Token']).toBeUndefined();
  });

  test('returns empty object when no token exists', () => {
    const headers = getCsrfHeaders();

    expect(Object.keys(headers).length).toBe(0);
  });

  test('can be spread into fetch headers', () => {
    setCookie('csrf_token', 'spread_token_vwx');

    const headers = {
      'Content-Type': 'application/json',
      ...getCsrfHeaders(),
    };

    expect(headers['Content-Type']).toBe('application/json');
    expect(headers['X-CSRF-Token']).toBe('spread_token_vwx');
  });
});

describe('Integration scenarios', () => {
  beforeEach(() => {
    clearCookies();
  });

  test('complete form submission workflow', async () => {
    setCookie('csrf_token', 'workflow_token_xyz');

    const { container } = render(
      <CsrfForm>
        <input name="username" defaultValue="testuser" />
        <button type="submit">Submit</button>
      </CsrfForm>
    );

    // Wait for CSRF input to render with correct value
    await waitFor(() => {
      const csrfInput = container.querySelector('input[type="hidden"][name="_csrf"]') as HTMLInputElement;
      expect(csrfInput).toBeTruthy();
      expect(csrfInput.value).toBe('workflow_token_xyz');
    });

    // Verify all inputs are present with correct values
    const csrfInput = container.querySelector('input[type="hidden"][name="_csrf"]') as HTMLInputElement;
    const usernameInput = container.querySelector('input[name="username"]') as HTMLInputElement;

    expect(csrfInput.value).toBe('workflow_token_xyz');
    expect(usernameInput.value).toBe('testuser');
  });

  test('manual FormData workflow', () => {
    setCookie('csrf_token', 'manual_workflow_token');

    const formData = new FormData();
    formData.append('email', 'test@example.com');
    addCsrfToFormData(formData);

    expect(formData.get('email')).toBe('test@example.com');
    expect(formData.get('_csrf')).toBe('manual_workflow_token');
  });
});
