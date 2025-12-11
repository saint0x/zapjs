import { motion } from 'framer-motion';
import { Zap, Github, Twitter, Heart } from 'lucide-react';

const footerLinks = [
  {
    title: 'Product',
    links: [
      { label: 'Features', href: '#features' },
      { label: 'Performance', href: '#performance' },
      { label: 'Architecture', href: '#architecture' },
      { label: 'Pricing', href: '#' },
    ],
  },
  {
    title: 'Developers',
    links: [
      { label: 'Documentation', href: '#' },
      { label: 'API Reference', href: '#' },
      { label: 'Examples', href: '#code' },
      { label: 'Changelog', href: '#' },
    ],
  },
  {
    title: 'Resources',
    links: [
      { label: 'Blog', href: '#' },
      { label: 'Community', href: '#' },
      { label: 'Discord', href: '#' },
      { label: 'Support', href: '#' },
    ],
  },
  {
    title: 'Company',
    links: [
      { label: 'About', href: '#' },
      { label: 'Careers', href: '#' },
      { label: 'Privacy', href: '#' },
      { label: 'Terms', href: '#' },
    ],
  },
];

export default function Footer() {
  return (
    <footer className="relative border-t border-carbon-800/50">
      {/* Background gradient */}
      <div className="absolute inset-0 bg-gradient-to-t from-carbon-950 to-transparent pointer-events-none" />

      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <div className="grid grid-cols-2 md:grid-cols-6 gap-8 lg:gap-12">
          {/* Brand column */}
          <div className="col-span-2">
            <motion.a
              href="#"
              className="inline-flex items-center gap-2 group"
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
            >
              <div className="relative">
                <div className="absolute inset-0 bg-zap-500/20 blur-xl rounded-full group-hover:bg-zap-500/30 transition-colors" />
                <div className="relative w-10 h-10 bg-gradient-to-br from-zap-400 to-zap-600 rounded-xl flex items-center justify-center shadow-lg shadow-zap-500/20">
                  <Zap className="w-6 h-6 text-white" fill="currentColor" />
                </div>
              </div>
              <span className="font-display font-bold text-2xl text-white">
                Zap<span className="text-zap-400">JS</span>
              </span>
            </motion.a>

            <p className="mt-4 text-carbon-400 text-sm leading-relaxed max-w-xs">
              Rust server. React frontend. Zero glue code.
            </p>

            {/* Social links */}
            <div className="mt-6 flex items-center gap-3">
              <a
                href="https://github.com/saint0x/zapjs"
                target="_blank"
                rel="noopener noreferrer"
                className="p-2 bg-carbon-800 hover:bg-carbon-700 rounded-lg transition-colors"
              >
                <Github className="w-5 h-5 text-carbon-400 hover:text-white" />
              </a>
              <a
                href="#"
                className="p-2 bg-carbon-800 hover:bg-carbon-700 rounded-lg transition-colors"
              >
                <Twitter className="w-5 h-5 text-carbon-400 hover:text-white" />
              </a>
              <a
                href="#"
                className="p-2 bg-carbon-800 hover:bg-carbon-700 rounded-lg transition-colors"
              >
                <svg viewBox="0 0 24 24" className="w-5 h-5 text-carbon-400 hover:text-white" fill="currentColor">
                  <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
                </svg>
              </a>
            </div>
          </div>

          {/* Link columns */}
          {footerLinks.map((group) => (
            <div key={group.title}>
              <h3 className="font-semibold text-white text-sm mb-4">{group.title}</h3>
              <ul className="space-y-3">
                {group.links.map((link) => (
                  <li key={link.label}>
                    <a
                      href={link.href}
                      className="text-sm text-carbon-400 hover:text-white transition-colors"
                    >
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        {/* Bottom bar */}
        <div className="mt-12 pt-8 border-t border-carbon-800/50 flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-sm text-carbon-500">
            &copy; {new Date().getFullYear()} ZapJS. All rights reserved.
          </p>

          <div className="flex items-center gap-1 text-sm text-carbon-500">
            Made with
            <Heart className="w-4 h-4 text-rust-400 mx-1" fill="currentColor" />
            using
            <span className="text-zap-400 font-medium ml-1">ZapJS</span>
          </div>

          <div className="flex items-center gap-4 text-sm">
            <a href="#" className="text-carbon-500 hover:text-white transition-colors">
              Privacy Policy
            </a>
            <a href="#" className="text-carbon-500 hover:text-white transition-colors">
              Terms of Service
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
