import { JSDOM } from 'jsdom';

// Set up jsdom FIRST, before any other imports
const dom = new JSDOM('<!DOCTYPE html><html><body></body></html>', {
  url: 'http://localhost',
  pretendToBeVisual: true,
  runScripts: 'dangerously',
});

// Set all the global properties from jsdom window
const { window } = dom;

// Copy all window properties to global
Object.defineProperty(global, 'window', {
  value: window,
  writable: true,
  configurable: true,
});

Object.defineProperty(global, 'document', {
  value: window.document,
  writable: true,
  configurable: true,
});

// Copy other essential globals
global.navigator = window.navigator as any;
global.HTMLElement = window.HTMLElement as any;
global.HTMLAnchorElement = window.HTMLAnchorElement as any;
global.HTMLButtonElement = window.HTMLButtonElement as any;
global.HTMLDivElement = window.HTMLDivElement as any;
global.Element = window.Element as any;
global.Node = window.Node as any;
global.DocumentFragment = window.DocumentFragment as any;
global.CustomEvent = window.CustomEvent as any;
global.Event = window.Event as any;
global.MouseEvent = window.MouseEvent as any;
global.KeyboardEvent = window.KeyboardEvent as any;

// Now import testing library after globals are set
import { afterEach, mock } from 'bun:test';
import { cleanup, configure } from '@testing-library/react';
import '@testing-library/jest-dom';

// Configure testing library
configure({ testIdAttribute: 'data-testid' });

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Mock console methods to reduce noise in tests
global.console = {
  ...console,
  log: mock(() => {}),
  debug: mock(() => {}),
  info: mock(() => {}),
  warn: mock(() => {}),
  error: mock(() => {}),
};
