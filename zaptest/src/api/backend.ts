// Auto-generated TypeScript runtime bindings
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

export const backend = {
  async listUsers(limit: number, offset: number): Promise<ListUsersResponse | ApiError> {
    return rpcCall<ListUsersResponse | ApiError>('list_users', { limit: limit, offset: offset });
  },

  async getUser(id: string): Promise<User | ApiError> {
    return rpcCall<User | ApiError>('get_user', { id: id });
  },

  async createUser(name: string, email: string, role: string): Promise<User | ApiError> {
    return rpcCall<User | ApiError>('create_user', { name: name, email: email, role: role });
  },

  async updateUser(id: string, name: string | null, email: string | null, role: string | null): Promise<User | ApiError> {
    return rpcCall<User | ApiError>('update_user', { id: id, name: name, email: email, role: role });
  },

  async deleteUser(id: string): Promise<DeleteUserResponse | ApiError> {
    return rpcCall<DeleteUserResponse | ApiError>('delete_user', { id: id });
  },

  async listPosts(page: number, limit: number, tag: string | null, author: string | null): Promise<ListPostsResponse | ApiError> {
    return rpcCall<ListPostsResponse | ApiError>('list_posts', { page: page, limit: limit, tag: tag, author: author });
  },

  async getPost(id: string): Promise<PostWithRelated | ApiError> {
    return rpcCall<PostWithRelated | ApiError>('get_post', { id: id });
  },

  async getStats(): Promise<StatsResponse | ApiError> {
    return rpcCall<StatsResponse | ApiError>('get_stats', {  });
  },

  async getFeatures(): Promise<FeaturesResponse | ApiError> {
    return rpcCall<FeaturesResponse | ApiError>('get_features', {  });
  },

  async getBenchmarks(): Promise<BenchmarksResponse | ApiError> {
    return rpcCall<BenchmarksResponse | ApiError>('get_benchmarks', {  });
  },

  async subscribe(email: string): Promise<SubscribeResponse | ApiError> {
    return rpcCall<SubscribeResponse | ApiError>('subscribe', { email: email });
  },

  async echoRequest(method: string, url: string, query: Record<string, string>, headers: Record<string, string>, body: string | null): Promise<EchoResponse | ApiError> {
    return rpcCall<EchoResponse | ApiError>('echo_request', { method: method, url: url, query: query, headers: headers, body: body });
  },

  async hello(): Promise<HelloResponse | ApiError> {
    return rpcCall<HelloResponse | ApiError>('hello', {  });
  },

  async getStreamingInfo(): Promise<StreamingInfo | ApiError> {
    return rpcCall<StreamingInfo | ApiError>('get_streaming_info', {  });
  },

  async getWebsocketInfo(): Promise<WebSocketInfo | ApiError> {
    return rpcCall<WebSocketInfo | ApiError>('get_websocket_info', {  });
  },

  async getSsgInfo(): Promise<SSGInfo | ApiError> {
    return rpcCall<SSGInfo | ApiError>('get_ssg_info', {  });
  },

};

export const listUsers = backend.listUsers;
export const getUser = backend.getUser;
export const createUser = backend.createUser;
export const updateUser = backend.updateUser;
export const deleteUser = backend.deleteUser;
export const listPosts = backend.listPosts;
export const getPost = backend.getPost;
export const getStats = backend.getStats;
export const getFeatures = backend.getFeatures;
export const getBenchmarks = backend.getBenchmarks;
export const subscribe = backend.subscribe;
export const echoRequest = backend.echoRequest;
export const hello = backend.hello;
export const getStreamingInfo = backend.getStreamingInfo;
export const getWebsocketInfo = backend.getWebsocketInfo;
export const getSsgInfo = backend.getSsgInfo;
