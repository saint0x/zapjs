/**
 * Blog Post Page with SSG
 *
 * Demonstrates generateStaticParams for Static Site Generation.
 * At build time, ZapJS calls generateStaticParams to get all slugs,
 * then pre-renders each page as static HTML.
 */

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Calendar, Clock, ArrowLeft, Tag } from 'lucide-react';
import { rpcCall } from '../../src/generated/rpc-client';
import { Link } from '../../src/router';

interface Post {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  content: string;
  author: string;
  tags: string[];
  publishedAt: string;
  readTime: string;
  relatedPosts: Array<{
    id: string;
    title: string;
    slug: string;
    excerpt: string;
  }>;
}

interface ListPostsResponse {
  posts: Array<{ slug: string }>;
}

/**
 * SSG: Generate static params at build time
 *
 * This function is called during the build process to determine
 * which paths should be pre-rendered. Each returned object
 * represents a page that will be statically generated.
 */
export async function generateStaticParams(): Promise<Array<{ slug: string }>> {
  try {
    const response = await rpcCall<ListPostsResponse>('list_posts', {
      page: 1,
      limit: 100,
      tag: null,
      author: null
    });

    return response.posts.map((post) => ({
      slug: post.slug
    }));
  } catch (error) {
    console.error('[SSG] Failed to generate static params:', error);
    return [];
  }
}

interface Props {
  params: { slug: string };
}

export default function BlogPost({ params }: Props) {
  const [post, setPost] = useState<Post | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchPost = async () => {
      try {
        const response = await rpcCall<Post>('get_post', { id: params.slug });
        setPost(response);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load post');
      } finally {
        setLoading(false);
      }
    };
    fetchPost();
  }, [params.slug]);

  if (loading) {
    return (
      <div className="min-h-screen bg-carbon-950 flex items-center justify-center">
        <div className="animate-spin w-8 h-8 border-2 border-zap-500 border-t-transparent rounded-full" />
      </div>
    );
  }

  if (error || !post) {
    return (
      <div className="min-h-screen bg-carbon-950 flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-2xl text-white mb-4">Post not found</h1>
          <p className="text-carbon-400 mb-6">{error || 'The requested post could not be found.'}</p>
          <Link
            to="/blog"
            className="inline-flex items-center gap-2 px-4 py-2 bg-zap-500 text-white rounded-lg hover:bg-zap-400 transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Blog
          </Link>
        </div>
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
      <article className="relative z-10 max-w-3xl mx-auto px-4 py-24">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
        >
          {/* Back link */}
          <Link
            to="/blog"
            className="inline-flex items-center gap-2 text-carbon-400 hover:text-white mb-8 transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Blog
          </Link>

          {/* Header */}
          <header className="mb-8">
            <div className="flex items-center gap-4 text-sm text-carbon-500 mb-4">
              <span className="flex items-center gap-1">
                <Calendar className="w-4 h-4" />
                {new Date(post.publishedAt).toLocaleDateString('en-US', {
                  year: 'numeric',
                  month: 'long',
                  day: 'numeric'
                })}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="w-4 h-4" />
                {post.readTime}
              </span>
            </div>

            <h1 className="text-4xl font-bold text-white mb-4">{post.title}</h1>

            <p className="text-lg text-carbon-400 mb-6">{post.excerpt}</p>

            <div className="flex items-center gap-4">
              <span className="text-carbon-400">By {post.author}</span>
              <div className="flex gap-2">
                {post.tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-2 py-1 text-xs bg-zap-500/10 text-zap-400 rounded-md"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          </header>

          {/* Content */}
          <div className="prose prose-invert prose-zap max-w-none">
            <div
              className="text-carbon-300 leading-relaxed whitespace-pre-wrap"
              dangerouslySetInnerHTML={{ __html: post.content }}
            />
          </div>

          {/* Related Posts */}
          {post.relatedPosts.length > 0 && (
            <div className="mt-16 pt-8 border-t border-carbon-800">
              <h2 className="text-xl font-semibold text-white mb-6">Related Posts</h2>
              <div className="grid gap-4">
                {post.relatedPosts.map((related) => (
                  <Link
                    key={related.id}
                    to={`/blog/${related.slug}`}
                    className="block p-4 bg-carbon-900/50 border border-carbon-800 rounded-lg hover:border-zap-500/50 transition-all duration-300"
                  >
                    <h3 className="text-white font-medium mb-1 hover:text-zap-400 transition-colors">
                      {related.title}
                    </h3>
                    <p className="text-sm text-carbon-400">{related.excerpt}</p>
                  </Link>
                ))}
              </div>
            </div>
          )}
        </motion.div>
      </article>
    </div>
  );
}

/**
 * Error Component for SSG
 *
 * Shown when the route throws an error or the post doesn't exist.
 */
export function errorComponent({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <div className="min-h-screen bg-carbon-950 flex items-center justify-center">
      <div className="text-center max-w-md">
        <h1 className="text-2xl text-white mb-4">Something went wrong</h1>
        <p className="text-carbon-400 mb-6">{error.message}</p>
        <div className="flex gap-4 justify-center">
          <button
            onClick={reset}
            className="px-4 py-2 bg-zap-500 text-white rounded-lg hover:bg-zap-400 transition-colors"
          >
            Try Again
          </button>
          <Link
            to="/blog"
            className="px-4 py-2 border border-carbon-700 text-carbon-300 rounded-lg hover:bg-carbon-800 transition-colors"
          >
            Back to Blog
          </Link>
        </div>
      </div>
    </div>
  );
}
