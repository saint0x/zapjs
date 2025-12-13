// Auto-generated TypeScript definitions
// DO NOT EDIT MANUALLY

import type {
  ApiError,
  BenchmarksResponse,
  DeleteUserResponse,
  EchoResponse,
  FeaturesResponse,
  HelloResponse,
  ListPostsResponse,
  ListUsersResponse,
  PostWithRelated,
  SSGInfo,
  StatsResponse,
  StreamingInfo,
  SubscribeResponse,
  User,
  WebSocketInfo,
} from './types';

// Re-export types for consumers
export * from './types';

/**
 * List all users with pagination
 */
export function list_users(limit: number, offset: number): Promise<ListUsersResponse | ApiError>;

/**
 * Get a single user by ID
 */
export function get_user(id: string): Promise<User | ApiError>;

/**
 * Create a new user
 */
export function create_user(name: string, email: string, role: string): Promise<User | ApiError>;

/**
 * Update an existing user
 */
export function update_user(id: string, name: string | null, email: string | null, role: string | null): Promise<User | ApiError>;

/**
 * Delete a user
 */
export function delete_user(id: string): Promise<DeleteUserResponse | ApiError>;

/**
 * List posts with pagination and filters
 */
export function list_posts(page: number, limit: number, tag: string | null, author: string | null): Promise<ListPostsResponse | ApiError>;

/**
 * Get a single post by ID or slug
 */
export function get_post(id: string): Promise<PostWithRelated | ApiError>;

/**
 * Get site statistics
 */
export function get_stats(): Promise<StatsResponse | ApiError>;

/**
 * Get features list
 */
export function get_features(): Promise<FeaturesResponse | ApiError>;

/**
 * Get benchmark data
 */
export function get_benchmarks(): Promise<BenchmarksResponse | ApiError>;

/**
 * Subscribe to newsletter
 */
export function subscribe(email: string): Promise<SubscribeResponse | ApiError>;

/**
 * Echo request details (for debugging)
 */
export function echo_request(method: string, url: string, query: Record<string, string>, headers: Record<string, string>, body: string | null): Promise<EchoResponse | ApiError>;

/**
 * Hello endpoint
 */
export function hello(): Promise<HelloResponse | ApiError>;

/**
 * Get streaming endpoint info
 */
export function get_streaming_info(): Promise<StreamingInfo | ApiError>;

/**
 * Get WebSocket endpoint info
 */
export function get_websocket_info(): Promise<WebSocketInfo | ApiError>;

/**
 * Get SSG (Static Site Generation) info
 */
export function get_ssg_info(): Promise<SSGInfo | ApiError>;

export interface ZapBackend {
  listUsers(limit: number, offset: number): Promise<ListUsersResponse | ApiError>;
  getUser(id: string): Promise<User | ApiError>;
  createUser(name: string, email: string, role: string): Promise<User | ApiError>;
  updateUser(id: string, name: string | null, email: string | null, role: string | null): Promise<User | ApiError>;
  deleteUser(id: string): Promise<DeleteUserResponse | ApiError>;
  listPosts(page: number, limit: number, tag: string | null, author: string | null): Promise<ListPostsResponse | ApiError>;
  getPost(id: string): Promise<PostWithRelated | ApiError>;
  getStats(): Promise<StatsResponse | ApiError>;
  getFeatures(): Promise<FeaturesResponse | ApiError>;
  getBenchmarks(): Promise<BenchmarksResponse | ApiError>;
  subscribe(email: string): Promise<SubscribeResponse | ApiError>;
  echoRequest(method: string, url: string, query: Record<string, string>, headers: Record<string, string>, body: string | null): Promise<EchoResponse | ApiError>;
  hello(): Promise<HelloResponse | ApiError>;
  getStreamingInfo(): Promise<StreamingInfo | ApiError>;
  getWebsocketInfo(): Promise<WebSocketInfo | ApiError>;
  getSsgInfo(): Promise<SSGInfo | ApiError>;
}

export declare const backend: ZapBackend;
