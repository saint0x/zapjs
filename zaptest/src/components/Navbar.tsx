import { motion } from 'framer-motion';
import { useState, useEffect } from 'react';
import { Zap, Github, Menu, X } from 'lucide-react';
import { cn } from '../lib/utils';
import { Link, useRouter, usePathname } from '../router';

export default function Navbar() {
  const [isScrolled, setIsScrolled] = useState(false);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    const handleScroll = () => {
      setIsScrolled(window.scrollY > 20);
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const navLinks = [
    { label: 'Features', href: '#features', isRoute: false },
    { label: 'Performance', href: '#performance', isRoute: false },
    { label: 'Code', href: '#code', isRoute: false },
    { label: 'Examples', href: '/examples', isRoute: true },
    { label: 'Docs', href: '/docs', isRoute: true },
  ];

  return (
    <motion.header
      initial={{ y: -100 }}
      animate={{ y: 0 }}
      transition={{ type: 'spring', stiffness: 100, damping: 20 }}
      className={cn(
        'fixed top-0 left-0 right-0 z-50 transition-all duration-500',
        isScrolled
          ? 'bg-carbon-950/80 backdrop-blur-xl border-b border-carbon-800/50'
          : 'bg-transparent'
      )}
    >
      <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16 sm:h-20">
          {/* Logo */}
          <motion.a
            href="#"
            className="flex items-center gap-2 group"
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
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
          </motion.a>

          {/* Desktop Nav */}
          <div className="hidden md:flex items-center gap-1">
            {navLinks.map((link, i) =>
              link.isRoute ? (
                <motion.div
                  key={link.href}
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.1 }}
                >
                  <Link
                    to={link.href}
                    className={cn(
                      "px-4 py-2 text-sm font-medium transition-colors relative group block",
                      pathname === link.href ? "text-zap-400" : "text-carbon-400 hover:text-white"
                    )}
                  >
                    {link.label}
                    <span className="absolute inset-x-4 -bottom-px h-px bg-gradient-to-r from-transparent via-zap-500 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
                  </Link>
                </motion.div>
              ) : (
                <motion.a
                  key={link.href}
                  href={link.href}
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.1 }}
                  className="px-4 py-2 text-sm font-medium text-carbon-400 hover:text-white transition-colors relative group"
                >
                  {link.label}
                  <span className="absolute inset-x-4 -bottom-px h-px bg-gradient-to-r from-transparent via-zap-500 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
                </motion.a>
              )
            )}
          </div>

          {/* CTA Buttons */}
          <div className="flex items-center gap-3">
            <motion.a
              href="https://github.com/saint0x/zapjs"
              target="_blank"
              rel="noopener noreferrer"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.4 }}
              className="hidden sm:flex items-center gap-2 px-4 py-2 text-sm font-medium text-carbon-300 hover:text-white transition-colors"
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
            >
              <Github className="w-4 h-4" />
              <span>GitHub</span>
            </motion.a>

            <motion.a
              href="#get-started"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.5 }}
              className="hidden sm:flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-zap-500 to-zap-600 hover:from-zap-400 hover:to-zap-500 text-white text-sm font-semibold rounded-full shadow-lg shadow-zap-500/25 transition-all duration-300"
              whileHover={{ scale: 1.02, boxShadow: '0 20px 40px rgba(236, 117, 26, 0.3)' }}
              whileTap={{ scale: 0.98 }}
            >
              Get Started
            </motion.a>

            {/* Mobile menu button */}
            <button
              onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
              className="md:hidden p-2 text-carbon-400 hover:text-white transition-colors"
            >
              {isMobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
            </button>
          </div>
        </div>

        {/* Mobile Menu */}
        <motion.div
          initial={false}
          animate={{
            height: isMobileMenuOpen ? 'auto' : 0,
            opacity: isMobileMenuOpen ? 1 : 0,
          }}
          className="md:hidden overflow-hidden"
        >
          <div className="py-4 space-y-2 border-t border-carbon-800/50">
            {navLinks.map((link) =>
              link.isRoute ? (
                <Link
                  key={link.href}
                  to={link.href}
                  onClick={() => setIsMobileMenuOpen(false)}
                  className={cn(
                    "block px-4 py-2 hover:bg-carbon-800/50 rounded-lg transition-colors",
                    pathname === link.href ? "text-zap-400" : "text-carbon-400 hover:text-white"
                  )}
                >
                  {link.label}
                </Link>
              ) : (
                <a
                  key={link.href}
                  href={link.href}
                  onClick={() => setIsMobileMenuOpen(false)}
                  className="block px-4 py-2 text-carbon-400 hover:text-white hover:bg-carbon-800/50 rounded-lg transition-colors"
                >
                  {link.label}
                </a>
              )
            )}
            <div className="pt-2 flex flex-col gap-2">
              <a
                href="https://github.com/saint0x/zapjs"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center justify-center gap-2 px-4 py-2.5 border border-carbon-700 text-carbon-300 rounded-lg hover:bg-carbon-800/50 transition-colors"
              >
                <Github className="w-4 h-4" />
                <span>GitHub</span>
              </a>
              <a
                href="#get-started"
                onClick={() => setIsMobileMenuOpen(false)}
                className="flex items-center justify-center gap-2 px-4 py-2.5 bg-gradient-to-r from-zap-500 to-zap-600 text-white font-semibold rounded-lg"
              >
                Get Started
              </a>
            </div>
          </div>
        </motion.div>
      </nav>
    </motion.header>
  );
}
