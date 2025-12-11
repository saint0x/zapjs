/**
 * Blog Index Page
 *
 * Lists all blog posts with links to individual post pages.
 * Demonstrates fetching data from the Rust backend via RPC.
 */

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Calendar, Clock, Tag, ArrowRight } from 'lucide-react';
import { rpcCall } from '../../src/generated/rpc-client';
import { Link } from '../../src/router';

interface PostSummary {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  author: string;
  tags: string[];
  publishedAt: string;
  readTime: string;
}

interface ListPostsResponse {
  posts: PostSummary[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    pages: number;
    hasNext: boolean;
    hasPrev: boolean;
  };
}

export default function BlogIndex() {
  const [posts, setPosts] = useState<PostSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchPosts = async () => {
      try {
        const response = await rpcCall<ListPostsResponse>('list_posts', {
          page: 1,
          limit: 10
        });
        setPosts(response.posts);
      } catch (error) {
        console.error('Failed to fetch posts:', error);
      } finally {
        setLoading(false);
      }
    };
    fetchPosts();
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen bg-carbon-950 flex items-center justify-center">
        <div className="animate-spin w-8 h-8 border-2 border-zap-500 border-t-transparent rounded-full" />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-carbon-950">
      {/* Background */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-[800px] h-[800px] bg-zap-500/5 rounded-full blur-[128px]" />
      </div>

      {/* Content */}
      <div className="relative z-10 max-w-4xl mx-auto px-4 py-24">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          <h1 className="text-4xl font-bold text-white mb-4">Blog</h1>
          <p className="text-lg text-carbon-400 mb-12">
            Learn about ZapJS, fullstack development, and Rust + TypeScript patterns.
          </p>
        </motion.div>

        <div className="space-y-6">
          {posts.map((post, index) => (
            <motion.article
              key={post.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: index * 0.1 }}
              className="group"
            >
              <Link to={`/blog/${post.slug}`}>
                <div className="p-6 bg-carbon-900/50 border border-carbon-800 rounded-xl hover:border-zap-500/50 transition-all duration-300">
                  <div className="flex items-center gap-4 text-sm text-carbon-500 mb-3">
                    <span className="flex items-center gap-1">
                      <Calendar className="w-4 h-4" />
                      {new Date(post.publishedAt).toLocaleDateString()}
                    </span>
                    <span className="flex items-center gap-1">
                      <Clock className="w-4 h-4" />
                      {post.readTime}
                    </span>
                    <span className="text-carbon-600">by {post.author}</span>
                  </div>

                  <h2 className="text-xl font-semibold text-white mb-2 group-hover:text-zap-400 transition-colors">
                    {post.title}
                  </h2>

                  <p className="text-carbon-400 mb-4">{post.excerpt}</p>

                  <div className="flex items-center justify-between">
                    <div className="flex gap-2">
                      {post.tags.map((tag) => (
                        <span
                          key={tag}
                          className="px-2 py-1 text-xs bg-carbon-800 text-carbon-400 rounded-md"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>

                    <span className="flex items-center gap-1 text-sm text-zap-400 opacity-0 group-hover:opacity-100 transition-opacity">
                      Read more <ArrowRight className="w-4 h-4" />
                    </span>
                  </div>
                </div>
              </Link>
            </motion.article>
          ))}
        </div>

        <div className="mt-12 text-center">
          <Link
            to="/"
            className="text-carbon-400 hover:text-white transition-colors"
          >
            Back to Home
          </Link>
        </div>
      </div>
    </div>
  );
}
