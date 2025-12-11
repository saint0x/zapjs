import { motion, useScroll, useTransform } from 'framer-motion';
import { useRef, lazy, Suspense } from 'react';
import { AppRouterProvider, usePathname } from './router';
import Navbar from './components/Navbar';
import Hero from './components/Hero';
import Features from './components/Features';
import Performance from './components/Performance';
import CodeDemo from './components/CodeDemo';
import Architecture from './components/Architecture';
import GetStarted from './components/GetStarted';
import Footer from './components/Footer';
import DocsPage from './pages/DocsPage';
import ExamplesPage from './pages/ExamplesPage';

// Lazy load blog pages
const BlogIndex = lazy(() => import('../routes/blog/index'));
const BlogPost = lazy(() => import('../routes/blog/[slug]'));

function LoadingSpinner() {
  return (
    <div className="min-h-screen bg-carbon-950 flex items-center justify-center">
      <div className="animate-spin w-8 h-8 border-2 border-zap-500 border-t-transparent rounded-full" />
    </div>
  );
}

function AppContent() {
  const currentPath = usePathname();

  // Render docs page if on /docs route
  if (currentPath === '/docs' || currentPath.startsWith('/docs')) {
    return <DocsPage />;
  }

  // Render examples page if on /examples route
  if (currentPath === '/examples' || currentPath.startsWith('/examples')) {
    return <ExamplesPage />;
  }

  // Render blog pages
  if (currentPath === '/blog') {
    return (
      <Suspense fallback={<LoadingSpinner />}>
        <BlogIndex />
      </Suspense>
    );
  }

  if (currentPath.startsWith('/blog/')) {
    const slug = currentPath.replace('/blog/', '');
    return (
      <Suspense fallback={<LoadingSpinner />}>
        <BlogPost params={{ slug }} />
      </Suspense>
    );
  }

  const containerRef = useRef<HTMLDivElement>(null);
  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ['start start', 'end end'],
  });

  const backgroundY = useTransform(scrollYProgress, [0, 1], ['0%', '100%']);

  return (
    <div ref={containerRef} className="relative min-h-screen bg-carbon-950 overflow-x-hidden">
      {/* Animated background gradient */}
      <motion.div
        className="fixed inset-0 pointer-events-none"
        style={{ y: backgroundY }}
      >
        <div className="absolute top-0 left-1/4 w-[800px] h-[800px] bg-zap-500/10 rounded-full blur-[128px] animate-pulse-slow" />
        <div className="absolute top-1/3 right-1/4 w-[600px] h-[600px] bg-rust-500/10 rounded-full blur-[128px] animate-pulse-slow" style={{ animationDelay: '2s' }} />
        <div className="absolute bottom-1/4 left-1/3 w-[500px] h-[500px] bg-zap-600/5 rounded-full blur-[128px] animate-pulse-slow" style={{ animationDelay: '4s' }} />
      </motion.div>

      {/* Grid overlay */}
      <div className="fixed inset-0 bg-grid pointer-events-none opacity-50" />

      {/* Noise texture */}
      <div className="fixed inset-0 pointer-events-none opacity-[0.015]" style={{
        backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 512 512' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")`,
      }} />

      {/* Content */}
      <div className="relative z-10">
        <Navbar />
        <main>
          <Hero />
          <Features />
          <Performance />
          <CodeDemo />
          <Architecture />
          <GetStarted />
        </main>
        <Footer />
      </div>

      {/* Built with Zap badge */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 2, duration: 0.5 }}
        className="fixed bottom-6 right-6 z-50"
      >
        <a
          href="https://github.com/saint0x/zapjs"
          target="_blank"
          rel="noopener noreferrer"
          className="group flex items-center gap-2 px-4 py-2 bg-carbon-900/80 backdrop-blur-sm border border-carbon-800 rounded-full text-sm text-carbon-400 hover:text-white hover:border-zap-500/50 transition-all duration-300"
        >
          <span className="relative flex h-2 w-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-zap-400 opacity-75"></span>
            <span className="relative inline-flex rounded-full h-2 w-2 bg-zap-500"></span>
          </span>
          <span>Built with <span className="text-zap-400 font-medium">Zap</span></span>
        </a>
      </motion.div>
    </div>
  );
}

function App() {
  return (
    <AppRouterProvider>
      <AppContent />
    </AppRouterProvider>
  );
}

export default App;
