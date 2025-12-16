/**
 * Streaming utilities for ZapJS
 * Helper functions for working with streaming responses
 */

import type { StreamChunk, StreamingHandler } from './types.js';
import { isAsyncIterable } from './types.js';

// Re-export type guard
export { isAsyncIterable };

/**
 * Create a StreamChunk from string data
 * @param data - String data to send
 * @returns StreamChunk with data field
 */
export function createChunk(data: string): StreamChunk;
/**
 * Create a StreamChunk from binary data
 * @param bytes - Binary data to send
 * @returns StreamChunk with bytes field
 */
export function createChunk(bytes: Uint8Array): StreamChunk;
export function createChunk(input: string | Uint8Array): StreamChunk {
  if (typeof input === 'string') {
    return { data: input };
  }
  return { bytes: input };
}

/**
 * Create a streaming response from an array of strings
 * @param items - Array of strings to stream
 * @param delimiter - Optional delimiter to add after each item (default: newline)
 * @returns Async iterable of stream chunks
 */
export async function* createStream(
  items: string[],
  delimiter: string = '\n'
): AsyncIterable<StreamChunk> {
  for (const item of items) {
    yield createChunk(item + delimiter);
  }
}

/**
 * Create a streaming JSON response (NDJSON format)
 * Each object is sent as a separate JSON line
 * @param objects - Array of objects to stream
 * @returns Async iterable of stream chunks
 */
export async function* streamJson<T = any>(objects: T[]): AsyncIterable<StreamChunk> {
  for (const obj of objects) {
    yield createChunk(JSON.stringify(obj) + '\n');
  }
}

/**
 * Stream Server-Sent Events (SSE) format
 * @param events - Array of SSE events
 * @returns Async iterable of stream chunks
 */
export async function* streamSSE(
  events: Array<{
    data: any;
    event?: string;
    id?: string | number;
    retry?: number;
  }>
): AsyncIterable<StreamChunk> {
  for (const evt of events) {
    let message = '';

    if (evt.event) {
      message += `event: ${evt.event}\n`;
    }
    if (evt.id !== undefined) {
      message += `id: ${evt.id}\n`;
    }
    if (evt.retry !== undefined) {
      message += `retry: ${evt.retry}\n`;
    }

    const data = typeof evt.data === 'string' ? evt.data : JSON.stringify(evt.data);
    message += `data: ${data}\n\n`;

    yield createChunk(message);
  }
}

/**
 * Transform an async iterable with a mapper function
 * @param source - Source async iterable
 * @param mapper - Function to transform each item
 * @returns Async iterable of transformed items
 */
export async function* mapStream<T, U>(
  source: AsyncIterable<T>,
  mapper: (item: T) => U | Promise<U>
): AsyncIterable<U> {
  for await (const item of source) {
    yield await mapper(item);
  }
}

/**
 * Filter an async iterable with a predicate function
 * @param source - Source async iterable
 * @param predicate - Function to test each item
 * @returns Async iterable of filtered items
 */
export async function* filterStream<T>(
  source: AsyncIterable<T>,
  predicate: (item: T) => boolean | Promise<boolean>
): AsyncIterable<T> {
  for await (const item of source) {
    if (await predicate(item)) {
      yield item;
    }
  }
}

/**
 * Batch stream chunks together
 * @param source - Source async iterable
 * @param batchSize - Number of items per batch
 * @returns Async iterable of batched items
 */
export async function* batchStream<T>(
  source: AsyncIterable<T>,
  batchSize: number
): AsyncIterable<T[]> {
  let batch: T[] = [];

  for await (const item of source) {
    batch.push(item);
    if (batch.length >= batchSize) {
      yield batch;
      batch = [];
    }
  }

  if (batch.length > 0) {
    yield batch;
  }
}

/**
 * Add delay between stream chunks
 * @param source - Source async iterable
 * @param delayMs - Delay in milliseconds between chunks
 * @returns Async iterable with delays
 */
export async function* delayStream<T>(
  source: AsyncIterable<T>,
  delayMs: number
): AsyncIterable<T> {
  for await (const item of source) {
    yield item;
    await new Promise(resolve => setTimeout(resolve, delayMs));
  }
}

/**
 * Convert a ReadableStream to an async iterable
 * @param stream - ReadableStream to convert
 * @returns Async iterable
 */
export async function* fromReadableStream<T>(
  stream: ReadableStream<T>
): AsyncIterable<T> {
  const reader = stream.getReader();
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      yield value;
    }
  } finally {
    reader.releaseLock();
  }
}

/**
 * Create a streaming response that emits at regular intervals
 * @param interval - Interval in milliseconds
 * @param maxCount - Maximum number of emissions (optional)
 * @param generator - Function to generate data for each emission
 * @returns Async iterable of stream chunks
 */
export async function* intervalStream<T>(
  interval: number,
  generator: (count: number) => T,
  maxCount?: number
): AsyncIterable<StreamChunk> {
  let count = 0;

  while (maxCount === undefined || count < maxCount) {
    const data = generator(count);
    const chunk = typeof data === 'string' ? data : JSON.stringify(data);
    yield createChunk(chunk);

    count++;
    if (maxCount !== undefined && count >= maxCount) break;

    await new Promise(resolve => setTimeout(resolve, interval));
  }
}
