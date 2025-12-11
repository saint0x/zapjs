import { motion } from 'framer-motion';
import { useState, useEffect } from 'react';
import {
  Zap,
  ChevronRight,
  ChevronLeft,
  Menu,
  X,
  Search,
  ExternalLink,
  Github
} from 'lucide-react';
import { cn } from '../../lib/utils';
import { Link, useRouter } from '../../router';

export interface DocSection {
  id: string;
  title: string;
  icon: React.ElementType;
  content: React.ReactNode;
}

interface DocsLayoutProps {
  sections: DocSection[];
  currentSection: string;
  onSectionChange: (sectionId: string) => void;
}

const sidebarCategories = [
  {
    label: 'Getting Started',
    items: ['introduction', 'quick-start', 'project-structure'],
  },
  {
    label: 'Core Concepts',
    items: ['architecture', 'routing', 'client-router', 'ssg', 'api-routes'],
  },
  {
    label: 'Data & RPC',
    items: ['enhanced-rpc', 'server-functions'],
  },
  {
    label: 'Production',
    items: ['security', 'observability', 'error-handling', 'caching', 'reliability'],
  },
  {
    label: 'Deploy',
    items: ['performance', 'deployment'],
  },
];

export default function DocsLayout({ sections, currentSection, onSectionChange }: DocsLayoutProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [isScrolled, setIsScrolled] = useState(false);
  const router = useRouter();

  useEffect(() => {
    const handleScroll = () => {
      setIsScrolled(window.scrollY > 20);
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const currentIndex = sections.findIndex(s => s.id === currentSection);
  const prevSection = currentIndex > 0 ? sections[currentIndex - 1] : null;
  const nextSection = currentIndex < sections.length - 1 ? sections[currentIndex + 1] : null;
  const activeSection = sections.find(s => s.id === currentSection);

  const filteredSections = searchQuery
    ? sections.filter(s =>
        s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.id.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : null;

  return (
    <div className="min-h-screen bg-carbon-950">
      {/* Desktop Header - similar to main page navbar */}
      <motion.header
        initial={{ y: -100 }}
        animate={{ y: 0 }}
        transition={{ type: 'spring', stiffness: 100, damping: 20 }}
        className={cn(
          'hidden lg:block fixed top-0 left-0 right-0 z-50 transition-all duration-500',
          isScrolled
            ? 'bg-carbon-950/80 backdrop-blur-xl border-b border-carbon-800/50'
            : 'bg-transparent'
        )}
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

            {/* Doc Section Navigation */}
            <div className="flex items-center gap-1 overflow-x-auto">
              {sections.slice(0, 6).map((section, i) => (
                <motion.button
                  key={section.id}
                  onClick={() => onSectionChange(section.id)}
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.05 }}
                  className={cn(
                    'px-3 py-2 text-sm font-medium transition-colors relative group whitespace-nowrap',
                    currentSection === section.id
                      ? 'text-zap-400'
                      : 'text-carbon-400 hover:text-white'
                  )}
                >
                  {section.title}
                  <span className={cn(
                    'absolute inset-x-3 -bottom-px h-px bg-gradient-to-r from-transparent via-zap-500 to-transparent transition-opacity',
                    currentSection === section.id ? 'opacity-100' : 'opacity-0 group-hover:opacity-50'
                  )} />
                </motion.button>
              ))}
              {sections.length > 6 && (
                <div className="relative group">
                  <button className="px-3 py-2 text-sm font-medium text-carbon-400 hover:text-white transition-colors">
                    More...
                  </button>
                  <div className="absolute right-0 top-full mt-2 py-2 bg-carbon-900/95 backdrop-blur-xl border border-carbon-800/50 rounded-xl shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all">
                    {sections.slice(6).map((section) => (
                      <button
                        key={section.id}
                        onClick={() => onSectionChange(section.id)}
                        className={cn(
                          'w-full px-4 py-2 text-sm text-left transition-colors whitespace-nowrap',
                          currentSection === section.id
                            ? 'text-zap-400 bg-zap-500/10'
                            : 'text-carbon-400 hover:text-white hover:bg-carbon-800/50'
                        )}
                      >
                        {section.title}
                      </button>
                    ))}
                  </div>
                </div>
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
                className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-carbon-300 hover:text-white transition-colors"
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
              >
                <Github className="w-4 h-4" />
                <span>GitHub</span>
              </motion.a>

              <Link
                to="/"
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-zap-500 to-zap-600 hover:from-zap-400 hover:to-zap-500 text-white text-sm font-semibold rounded-full shadow-lg shadow-zap-500/25 transition-all duration-300"
              >
                Back to Home
              </Link>
            </div>
          </div>
        </nav>
      </motion.header>

      {/* Mobile Header */}
      <div className="lg:hidden fixed top-0 left-0 right-0 z-50 bg-carbon-950/95 backdrop-blur-xl border-b border-carbon-800/50">
        <div className="flex items-center justify-between px-4 h-16">
          <button
            onClick={() => setSidebarOpen(true)}
            className="p-2 text-carbon-400 hover:text-white transition-colors"
          >
            <Menu className="w-6 h-6" />
          </button>
          <Link
            to="/"
            className="flex items-center gap-2"
          >
            <div className="w-7 h-7 bg-gradient-to-br from-zap-400 to-zap-600 rounded-lg flex items-center justify-center">
              <Zap className="w-4 h-4 text-white" fill="currentColor" />
            </div>
            <span className="font-display font-bold text-white">Zap<span className="text-zap-400">JS</span> Docs</span>
          </Link>
          <div className="w-10" />
        </div>
        {/* Mobile section tabs */}
        <div className="flex overflow-x-auto px-2 pb-2 gap-1 hide-scrollbar">
          {sections.map((section) => (
            <button
              key={section.id}
              onClick={() => onSectionChange(section.id)}
              className={cn(
                'px-3 py-1.5 text-xs font-medium rounded-full whitespace-nowrap transition-colors',
                currentSection === section.id
                  ? 'bg-zap-500/20 text-zap-400 border border-zap-500/30'
                  : 'text-carbon-400 bg-carbon-800/30 hover:bg-carbon-800/50'
              )}
            >
              {section.title}
            </button>
          ))}
        </div>
      </div>

      {/* Mobile Sidebar Overlay */}
      {sidebarOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 bg-black/60 z-50 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Mobile Sidebar */}
      <motion.aside
        initial={false}
        animate={{ x: sidebarOpen ? 0 : '-100%' }}
        className="fixed top-0 left-0 bottom-0 w-80 bg-carbon-950 border-r border-carbon-800/50 z-50 lg:hidden"
      >
        <div className="flex flex-col h-full">
          {/* Sidebar Header */}
          <div className="flex items-center justify-between p-4 border-b border-carbon-800/50">
            <Link
              to="/"
              className="flex items-center gap-2 group"
            >
              <div className="relative">
                <div className="absolute inset-0 bg-zap-500/20 blur-xl rounded-full" />
                <div className="relative w-8 h-8 bg-gradient-to-br from-zap-400 to-zap-600 rounded-lg flex items-center justify-center">
                  <Zap className="w-4 h-4 text-white" fill="currentColor" />
                </div>
              </div>
              <span className="font-display font-bold text-lg text-white">
                Zap<span className="text-zap-400">JS</span>
              </span>
            </Link>
            <button
              onClick={() => setSidebarOpen(false)}
              className="p-2 text-carbon-400 hover:text-white transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          {/* Search */}
          <div className="p-4">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-carbon-500" />
              <input
                type="text"
                placeholder="Search documentation..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-10 pr-4 py-2.5 bg-carbon-900/50 border border-carbon-800 rounded-lg text-sm text-white placeholder:text-carbon-500 focus:outline-none focus:border-zap-500/50 transition-colors"
              />
            </div>
          </div>

          {/* Search Results */}
          {filteredSections && (
            <div className="px-4 pb-4">
              <div className="bg-carbon-900/50 border border-carbon-800 rounded-lg overflow-hidden">
                {filteredSections.length === 0 ? (
                  <p className="px-4 py-3 text-sm text-carbon-500">No results found</p>
                ) : (
                  filteredSections.map(section => (
                    <button
                      key={section.id}
                      onClick={() => {
                        onSectionChange(section.id);
                        setSearchQuery('');
                        setSidebarOpen(false);
                      }}
                      className="w-full flex items-center gap-3 px-4 py-3 text-left text-sm text-carbon-300 hover:text-white hover:bg-carbon-800/50 transition-colors"
                    >
                      <section.icon className="w-4 h-4 text-carbon-500" />
                      {section.title}
                    </button>
                  ))
                )}
              </div>
            </div>
          )}

          {/* Navigation */}
          <nav className="flex-1 overflow-y-auto px-4 pb-4">
            {sidebarCategories.map(category => (
              <div key={category.label} className="mb-6">
                <h3 className="px-3 mb-2 text-xs font-semibold text-carbon-500 uppercase tracking-wider">
                  {category.label}
                </h3>
                <ul className="space-y-1">
                  {category.items.map(itemId => {
                    const section = sections.find(s => s.id === itemId);
                    if (!section) return null;
                    const isActive = currentSection === itemId;

                    return (
                      <li key={itemId}>
                        <button
                          onClick={() => {
                            onSectionChange(itemId);
                            setSidebarOpen(false);
                          }}
                          className={cn(
                            'w-full flex items-center gap-3 px-3 py-2 text-sm rounded-lg transition-all duration-200',
                            isActive
                              ? 'bg-zap-500/10 text-zap-400 border border-zap-500/20'
                              : 'text-carbon-400 hover:text-white hover:bg-carbon-800/50'
                          )}
                        >
                          <section.icon className={cn('w-4 h-4', isActive ? 'text-zap-400' : 'text-carbon-500')} />
                          {section.title}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </div>
            ))}
          </nav>

          {/* Footer Link */}
          <div className="p-4 border-t border-carbon-800/50">
            <a
              href="https://github.com/saint0x/zapjs"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-sm text-carbon-400 hover:text-white transition-colors"
            >
              <ExternalLink className="w-4 h-4" />
              View on GitHub
            </a>
          </div>
        </div>
      </motion.aside>

      {/* Main Content */}
      <main className="min-h-screen pt-28 lg:pt-32">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8 lg:py-12">
          {/* Breadcrumb */}
          <div className="flex items-center gap-2 text-sm text-carbon-500 mb-8">
            <Link
              to="/"
              className="hover:text-white transition-colors"
            >Home</Link>
            <ChevronRight className="w-4 h-4" />
            <span className="text-carbon-300">Docs</span>
            <ChevronRight className="w-4 h-4" />
            <span className="text-zap-400">{activeSection?.title}</span>
          </div>

          {/* Content */}
          <motion.div
            key={currentSection}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3 }}
          >
            {activeSection?.content}
          </motion.div>

          {/* Pagination */}
          <div className="flex items-center justify-between mt-16 pt-8 border-t border-carbon-800/50">
            {prevSection ? (
              <button
                onClick={() => onSectionChange(prevSection.id)}
                className="group flex items-center gap-3 px-4 py-3 bg-carbon-900/30 border border-carbon-800/50 rounded-xl hover:border-carbon-700/50 transition-all"
              >
                <ChevronLeft className="w-5 h-5 text-carbon-500 group-hover:text-zap-400 transition-colors" />
                <div className="text-left">
                  <p className="text-xs text-carbon-500 mb-1">Previous</p>
                  <p className="text-sm font-medium text-white">{prevSection.title}</p>
                </div>
              </button>
            ) : <div />}

            {nextSection ? (
              <button
                onClick={() => onSectionChange(nextSection.id)}
                className="group flex items-center gap-3 px-4 py-3 bg-carbon-900/30 border border-carbon-800/50 rounded-xl hover:border-carbon-700/50 transition-all"
              >
                <div className="text-right">
                  <p className="text-xs text-carbon-500 mb-1">Next</p>
                  <p className="text-sm font-medium text-white">{nextSection.title}</p>
                </div>
                <ChevronRight className="w-5 h-5 text-carbon-500 group-hover:text-zap-400 transition-colors" />
              </button>
            ) : <div />}
          </div>
        </div>
      </main>
    </div>
  );
}
