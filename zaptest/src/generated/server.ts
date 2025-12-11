// Auto-generated server client
// DO NOT EDIT MANUALLY

import { rpcCall } from './rpc-client';
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

export const server = {
  default: {
    async listUsers(params: { limit: number, offset: number }): Promise<ListUsersResponse | ApiError> {
      return rpcCall<ListUsersResponse | ApiError>('list_users', { limit: params.limit, offset: params.offset });
    },
    async getUser(params: { id: string }): Promise<User | ApiError> {
      return rpcCall<User | ApiError>('get_user', { id: params.id });
    },
    async createUser(params: { name: string, email: string, role: string }): Promise<User | ApiError> {
      return rpcCall<User | ApiError>('create_user', { name: params.name, email: params.email, role: params.role });
    },
    async updateUser(params: { id: string, name: string | null, email: string | null, role: string | null }): Promise<User | ApiError> {
      return rpcCall<User | ApiError>('update_user', { id: params.id, name: params.name, email: params.email, role: params.role });
    },
    async deleteUser(params: { id: string }): Promise<DeleteUserResponse | ApiError> {
      return rpcCall<DeleteUserResponse | ApiError>('delete_user', { id: params.id });
    },
    async listPosts(params: { page: number, limit: number, tag: string | null, author: string | null }): Promise<ListPostsResponse | ApiError> {
      return rpcCall<ListPostsResponse | ApiError>('list_posts', { page: params.page, limit: params.limit, tag: params.tag, author: params.author });
    },
    async getPost(params: { id: string }): Promise<PostWithRelated | ApiError> {
      return rpcCall<PostWithRelated | ApiError>('get_post', { id: params.id });
    },
    async getStats(): Promise<StatsResponse | ApiError> {
      return rpcCall<StatsResponse | ApiError>('get_stats', {});
    },
    async getFeatures(): Promise<FeaturesResponse | ApiError> {
      return rpcCall<FeaturesResponse | ApiError>('get_features', {});
    },
    async getBenchmarks(): Promise<BenchmarksResponse | ApiError> {
      return rpcCall<BenchmarksResponse | ApiError>('get_benchmarks', {});
    },
    async subscribe(params: { email: string }): Promise<SubscribeResponse | ApiError> {
      return rpcCall<SubscribeResponse | ApiError>('subscribe', { email: params.email });
    },
    async echoRequest(params: { method: string, url: string, query: Record<string, string>, headers: Record<string, string>, body: string | null }): Promise<EchoResponse | ApiError> {
      return rpcCall<EchoResponse | ApiError>('echo_request', { method: params.method, url: params.url, query: params.query, headers: params.headers, body: params.body });
    },
    async hello(): Promise<HelloResponse | ApiError> {
      return rpcCall<HelloResponse | ApiError>('hello', {});
    },
    async getStreamingInfo(): Promise<StreamingInfo | ApiError> {
      return rpcCall<StreamingInfo | ApiError>('get_streaming_info', {});
    },
    async getWebsocketInfo(): Promise<WebSocketInfo | ApiError> {
      return rpcCall<WebSocketInfo | ApiError>('get_websocket_info', {});
    },
    async getSsgInfo(): Promise<SSGInfo | ApiError> {
      return rpcCall<SSGInfo | ApiError>('get_ssg_info', {});
    },
  },
} as const;

export type Server = typeof server;
