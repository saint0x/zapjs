// Auto-generated TypeScript interfaces
// DO NOT EDIT MANUALLY

export interface User {
  id: string;
  name: string;
  email: string;
  role: string;
  createdAt: string;
}

export interface Post {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  content: string;
  author: string;
  tags: string[];
  publishedAt: string;
  readTime: string;
}

export interface PostSummary {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  author: string;
  tags: string[];
  publishedAt: string;
  readTime: string;
}

export interface Feature {
  id: string;
  icon: string;
  title: string;
  description: string;
  color: string;
}

export interface Benchmark {
  name: string;
  requestsPerSec: number;
  latencyMs: number;
  isHighlighted: boolean;
}

export interface ListUsersResponse {
  users: User[];
  total: number;
  limit: number;
  offset: number;
  hasMore: boolean;
}

export interface ListPostsResponse {
  posts: PostSummary[];
  pagination: Pagination;
  filters: PostFilters;
}

export interface Pagination {
  page: number;
  limit: number;
  total: number;
  pages: number;
  hasNext: boolean;
  hasPrev: boolean;
}

export interface PostFilters {
  tag?: string | null;
  author?: string | null;
}

export interface StatsResponse {
  version: string;
  uptime: string;
  requests: number;
  responseTime: string;
  environment: string;
  serverStarted: string;
}

export interface FeaturesResponse {
  features: Feature[];
  count: number;
  lastUpdated: string;
}

export interface BenchmarksResponse {
  frameworks: Benchmark[];
  metrics: BenchmarkMetrics;
  machine: string;
  os: string;
  lastUpdated: string;
}

export interface BenchmarkMetrics {
  routeLookup: string;
  throughput: string;
  memory: string;
  p99Latency: string;
}

export interface PostWithRelated {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  content: string;
  author: string;
  tags: string[];
  publishedAt: string;
  readTime: string;
  relatedPosts: PostSummary[];
}

export interface SubscribeResponse {
  success: boolean;
  message: string;
  email: string;
  subscribedAt: string;
}

export interface EchoResponse {
  method: string;
  url: string;
  query: Record<string, string>;
  headers: Record<string, string>;
  body?: string | null;
  timestamp: string;
}

export interface HelloResponse {
  message: string;
  timestamp: string;
}

export interface DeleteUserResponse {
  success: boolean;
  message: string;
  deletedUser: User;
}

export interface ApiError {
  error: string;
  code: string;
}

export interface StreamingInfo {
  endpoint: string;
  supportedFormats: string[];
  maxChunkSize: number;
  description: string;
  exampleEvents: string[];
}

export interface WebSocketInfo {
  endpoint: string;
  protocol: string;
  commands: Record<string, string>;
  connectedClients: number;
  description: string;
}

export interface SSGInfo {
  staticRoutes: string[];
  totalPages: number;
  buildTime: string;
  description: string;
  generatedAt: string;
}

