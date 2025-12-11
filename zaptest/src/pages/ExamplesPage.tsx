import { motion } from 'framer-motion';
import { Zap, Github, ArrowLeft } from 'lucide-react';
import Examples from '../components/Examples';
import { Link, useRouter } from '../router';

export default function ExamplesPage() {
  const router = useRouter();

  return (
    <div className="min-h-screen bg-carbon-950">
      {/* Grid overlay */}
      <div className="fixed inset-0 bg-grid pointer-events-none opacity-50" />

      {/* Background gradients */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-[800px] h-[800px] bg-zap-500/10 rounded-full blur-[128px]" />
        <div className="absolute top-1/3 right-1/4 w-[600px] h-[600px] bg-rust-500/10 rounded-full blur-[128px]" />
      </div>

      {/* Header */}
      <motion.header
        initial={{ y: -100 }}
        animate={{ y: 0 }}
        transition={{ type: 'spring', stiffness: 100, damping: 20 }}
        className="fixed top-0 left-0 right-0 z-50 bg-carbon-950/80 backdrop-blur-xl border-b border-carbon-800/50"
      >
        <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16 sm:h-20">
            {/* Logo */}
            <Link to="/" className="flex items-center gap-2 group">
              <motion.div
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                className="flex items-center gap-2"
              >
                <div className="relative">
                  <div className="absolute inset-0 bg-zap-500/20 blur-xl rounded-full group-hover:bg-zap-500/30 transition-colors" />
                  <div className="relative w-9 h-9 sm:w-10 sm:h-10 bg-gradient-to-br from-zap-400 to-zap-600 rounded-xl flex items-center justify-center shadow-lg shadow-zap-500/20">
                    <Zap className="w-5 h-5 sm:w-6 sm:h-6 text-white" fill="currentColor" />
                  </div>
                </div>
                <span className="font-display font-bold text-xl sm:text-2xl text-white">
                  Zap<span className="text-zap-400">JS</span>
                </span>
              </motion.div>
            </Link>

            {/* Navigation Links */}
            <div className="flex items-center gap-6">
              <Link
                to="/docs"
                className="text-sm font-medium text-carbon-400 hover:text-white transition-colors"
              >
                Docs
              </Link>
              <span className="text-sm font-medium text-zap-400">Examples</span>
            </div>

            {/* CTA Buttons */}
            <div className="flex items-center gap-3">
              <motion.a
                href="https://github.com/saint0x/zapjs"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-carbon-300 hover:text-white transition-colors"
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
              >
                <Github className="w-4 h-4" />
                <span className="hidden sm:inline">GitHub</span>
              </motion.a>

              <Link
                to="/"
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-zap-500 to-zap-600 hover:from-zap-400 hover:to-zap-500 text-white text-sm font-semibold rounded-full shadow-lg shadow-zap-500/25 transition-all duration-300"
              >
                <ArrowLeft className="w-4 h-4" />
                Home
              </Link>
            </div>
          </div>
        </nav>
      </motion.header>

      {/* Content */}
      <div className="relative z-10 pt-24">
        <Examples />
      </div>
    </div>
  );
}
