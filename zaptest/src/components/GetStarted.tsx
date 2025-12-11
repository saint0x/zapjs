import { motion, useInView } from 'framer-motion';
import { useRef, useState } from 'react';
import { Rocket, Copy, Check, Terminal, ArrowRight, Folder, FileCode, Server } from 'lucide-react';

const steps = [
  {
    command: 'npx create-zap-app my-app',
    description: 'Scaffold a new project',
  },
  {
    command: 'cd my-app && bun install',
    description: 'Install dependencies',
  },
  {
    command: 'bun run dev',
    description: 'Start dev server',
  },
];

const projectStructure = [
  { type: 'folder', name: 'routes/', indent: 0, description: 'File-based routing' },
  { type: 'file', name: '__root.tsx', indent: 1, description: 'Root layout' },
  { type: 'file', name: 'index.tsx', indent: 1, description: 'Home page (/)' },
  { type: 'folder', name: 'api/', indent: 1, description: 'API routes' },
  { type: 'file', name: 'hello.ts', indent: 2, description: '/api/hello' },
  { type: 'folder', name: 'server/', indent: 0, description: 'Rust handlers' },
  { type: 'file', name: 'lib.rs', indent: 1, description: '#[zap::export] functions' },
  { type: 'folder', name: 'src/', indent: 0, description: 'React components' },
  { type: 'folder', name: 'generated/', indent: 1, description: 'Auto-generated types' },
  { type: 'file', name: 'package.json', indent: 0, description: 'Dependencies' },
  { type: 'file', name: 'Cargo.toml', indent: 0, description: 'Rust config' },
  { type: 'file', name: 'zap.config.ts', indent: 0, description: 'ZapJS config' },
];

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button
      onClick={handleCopy}
      className="p-2 hover:bg-carbon-700 rounded-lg transition-colors"
    >
      {copied ? (
        <Check className="w-4 h-4 text-emerald-400" />
      ) : (
        <Copy className="w-4 h-4 text-carbon-400" />
      )}
    </button>
  );
}

export default function GetStarted() {
  const ref = useRef<HTMLDivElement>(null);
  const isInView = useInView(ref, { once: true, margin: '-100px' });

  return (
    <section id="get-started" className="relative py-24 sm:py-32">
      {/* Background gradient */}
      <div className="absolute inset-0 bg-gradient-to-b from-transparent via-zap-500/5 to-transparent pointer-events-none" />

      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <motion.div
          ref={ref}
          initial={{ opacity: 0, y: 30 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.6 }}
          className="text-center mb-16"
        >
          <div className="inline-flex items-center gap-2 px-4 py-2 mb-6 bg-zap-500/10 border border-zap-500/20 rounded-full">
            <Rocket className="w-4 h-4 text-zap-400" />
            <span className="text-sm font-medium text-zap-400">Get Started</span>
          </div>

          <h2 className="font-display font-black text-4xl sm:text-5xl md:text-6xl text-white mb-6">
            Get{' '}
            <span className="text-gradient">started</span>
          </h2>

          <p className="text-lg text-carbon-400 max-w-2xl mx-auto">
            Three commands. Working fullstack app.
          </p>
        </motion.div>

        <div className="grid lg:grid-cols-2 gap-12 items-start">
          {/* Left: Terminal steps */}
          <motion.div
            initial={{ opacity: 0, x: -30 }}
            animate={isInView ? { opacity: 1, x: 0 } : {}}
            transition={{ duration: 0.6, delay: 0.2 }}
          >
            {/* Terminal window */}
            <div className="relative">
              <div className="absolute -inset-2 bg-gradient-to-r from-zap-500/20 to-rust-500/20 rounded-2xl blur-xl opacity-50" />

              <div className="relative bg-carbon-900/90 backdrop-blur-xl border border-carbon-800 rounded-2xl overflow-hidden">
                {/* Terminal header */}
                <div className="flex items-center gap-2 px-4 py-3 bg-carbon-900/50 border-b border-carbon-800">
                  <div className="w-3 h-3 rounded-full bg-red-500/80" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500/80" />
                  <div className="w-3 h-3 rounded-full bg-green-500/80" />
                  <span className="ml-3 text-sm text-carbon-500 font-mono">terminal</span>
                </div>

                {/* Commands */}
                <div className="p-4 sm:p-6 space-y-6">
                  {steps.map((step, i) => (
                    <motion.div
                      key={step.command}
                      initial={{ opacity: 0, x: -20 }}
                      animate={isInView ? { opacity: 1, x: 0 } : {}}
                      transition={{ duration: 0.4, delay: 0.3 + i * 0.15 }}
                    >
                      <div className="flex items-center justify-between gap-3 p-3 bg-carbon-800/50 rounded-lg group">
                        <div className="flex items-center gap-3 min-w-0">
                          <span className="text-emerald-400 font-mono">$</span>
                          <code className="font-mono text-sm text-white truncate">{step.command}</code>
                        </div>
                        <CopyButton text={step.command} />
                      </div>
                      <p className="mt-2 text-sm text-carbon-500 pl-6">{step.description}</p>
                    </motion.div>
                  ))}

                  {/* Success message */}
                  <motion.div
                    initial={{ opacity: 0 }}
                    animate={isInView ? { opacity: 1 } : {}}
                    transition={{ duration: 0.5, delay: 0.8 }}
                    className="pt-4 border-t border-carbon-800"
                  >
                    <div className="flex items-center gap-2 text-emerald-400">
                      <Check className="w-5 h-5" />
                      <span className="font-mono text-sm">Ready! Open http://localhost:3000</span>
                    </div>
                  </motion.div>
                </div>
              </div>
            </div>

            {/* Quick links */}
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={isInView ? { opacity: 1, y: 0 } : {}}
              transition={{ duration: 0.5, delay: 1 }}
              className="mt-8 flex flex-wrap gap-4"
            >
              <a
                href="https://github.com/saint0x/zapjs"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-2 px-5 py-2.5 bg-carbon-800 hover:bg-carbon-700 border border-carbon-700 text-white font-medium rounded-full transition-colors"
              >
                <svg viewBox="0 0 24 24" className="w-5 h-5" fill="currentColor">
                  <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                </svg>
                View on GitHub
              </a>
              <a
                href="#"
                className="inline-flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-zap-500 to-zap-600 text-white font-semibold rounded-full shadow-lg shadow-zap-500/25 hover:shadow-zap-500/40 transition-shadow"
              >
                Read the Docs
                <ArrowRight className="w-4 h-4" />
              </a>
            </motion.div>
          </motion.div>

          {/* Right: Project structure */}
          <motion.div
            initial={{ opacity: 0, x: 30 }}
            animate={isInView ? { opacity: 1, x: 0 } : {}}
            transition={{ duration: 0.6, delay: 0.3 }}
          >
            <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
              <Folder className="w-5 h-5 text-zap-400" />
              Project Structure
            </h3>

            <div className="bg-carbon-900/50 border border-carbon-800 rounded-xl p-4 sm:p-6">
              <div className="font-mono text-sm space-y-1">
                {projectStructure.map((item, i) => (
                  <motion.div
                    key={`${item.name}-${i}`}
                    initial={{ opacity: 0, x: -10 }}
                    animate={isInView ? { opacity: 1, x: 0 } : {}}
                    transition={{ duration: 0.3, delay: 0.4 + i * 0.05 }}
                    className="flex items-center gap-3 py-1 group"
                    style={{ paddingLeft: `${item.indent * 20}px` }}
                  >
                    {item.type === 'folder' ? (
                      <Folder className="w-4 h-4 text-zap-400 flex-shrink-0" />
                    ) : (
                      <FileCode className="w-4 h-4 text-carbon-500 flex-shrink-0" />
                    )}
                    <span className={item.type === 'folder' ? 'text-zap-400' : 'text-carbon-300'}>
                      {item.name}
                    </span>
                    <span className="text-carbon-600 text-xs opacity-0 group-hover:opacity-100 transition-opacity">
                      {item.description}
                    </span>
                  </motion.div>
                ))}
              </div>
            </div>

            {/* Feature callouts */}
            <div className="mt-6 grid grid-cols-2 gap-4">
              {[
                { icon: Terminal, label: 'CLI Tools', value: 'zap dev, build, routes' },
                { icon: Server, label: 'Hot Reload', value: 'Rust + TS' },
              ].map((item, i) => (
                <motion.div
                  key={item.label}
                  initial={{ opacity: 0, y: 10 }}
                  animate={isInView ? { opacity: 1, y: 0 } : {}}
                  transition={{ duration: 0.4, delay: 0.8 + i * 0.1 }}
                  className="p-4 bg-carbon-900/30 border border-carbon-800/50 rounded-xl"
                >
                  <item.icon className="w-5 h-5 text-zap-400 mb-2" />
                  <div className="text-sm font-medium text-white">{item.label}</div>
                  <div className="text-xs text-carbon-500">{item.value}</div>
                </motion.div>
              ))}
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}
