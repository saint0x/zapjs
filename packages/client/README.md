# @zapjs/client

Official CLI and development tools for ZapJS - the fullstack Rust + React framework.

## Installation

### Global Installation (Recommended)

Install the ZapJS CLI globally to use the `zap` command anywhere:

```bash
npm install -g @zapjs/client
```

After installation, verify it works:

```bash
zap --version
zap --help
```

### Local Project Installation

For project-specific installation:

```bash
npm install --save-dev @zapjs/client
```

Then use via npm scripts in your `package.json`:

```json
{
  "scripts": {
    "dev": "zap dev",
    "build": "zap build",
    "serve": "zap serve"
  }
}
```

### Local Development (Contributors)

If you're contributing to ZapJS:

```bash
cd packages/client
npm install
npm run build
npm link
```

This creates a global symlink for local testing. The `zap` command will now use your local development version.

## Available Commands

### `zap new <name>`

Create a new ZapJS project with the specified template.

```bash
zap new my-app
zap new my-app --template fullstack
zap new my-app --no-install --no-git
```

**Options:**
- `-t, --template <template>` - Template to use: `basic` or `fullstack` (default: `basic`)
- `--no-install` - Skip npm install
- `--no-git` - Skip git initialization

### `zap dev`

Start the development server with hot reload for both Rust and TypeScript.

```bash
zap dev
zap dev --port 3000 --vite-port 5173
zap dev --release --skip-build
```

**Options:**
- `-p, --port <port>` - API server port (default: `3000`)
- `--vite-port <port>` - Vite dev server port (default: `5173`)
- `--no-open` - Don't open browser automatically
- `-l, --log-level <level>` - Log level: `debug`, `info`, `warn`, `error` (default: `info`)
- `--release` - Build Rust in release mode (slower build, faster runtime)
- `--skip-build` - Skip initial Rust build (use existing binary)
- `--binary-path <path>` - Path to pre-built zap binary
- `--codegen-binary-path <path>` - Path to pre-built zap-codegen binary

**Keyboard Shortcuts (during dev):**
- `r` - Manually trigger Rust rebuild
- `c` - Regenerate TypeScript bindings
- `q` - Quit dev server
- `Ctrl+C` - Stop server

### `zap build`

Build your application for production.

```bash
zap build
zap build --output ./dist --target x86_64-unknown-linux-gnu
zap build --skip-frontend
```

**Options:**
- `--release` - Build optimized release (default: `true`)
- `-o, --output <dir>` - Output directory (default: `./dist`)
- `--target <target>` - Cross-compile target (e.g., `x86_64-unknown-linux-gnu`)
- `--skip-frontend` - Skip Vite frontend build
- `--skip-codegen` - Skip TypeScript binding generation

**Output Structure:**
```
dist/
├── bin/
│   └── zap              # Rust binary
├── static/              # Frontend assets (if built)
├── config.json          # Production config
└── manifest.json        # Build metadata
```

### `zap serve`

Run the production server.

```bash
zap serve
zap serve --port 8080 --host 0.0.0.0
zap serve --config ./custom-config.json
```

**Options:**
- `-p, --port <port>` - Server port
- `--host <host>` - Host to bind to (default: `0.0.0.0`)
- `-c, --config <path>` - Path to config file
- `-w, --workers <count>` - Number of worker threads

### `zap codegen`

Generate TypeScript bindings from Rust exports.

```bash
zap codegen
zap codegen --output ./src/api
zap codegen --input ./metadata.json
```

**Options:**
- `-i, --input <file>` - Input metadata JSON file
- `-o, --output <dir>` - Output directory (default: `./src/api`)

### `zap routes`

Scan routes directory and display/generate route tree.

```bash
zap routes
zap routes --routes-dir ./routes --output ./src/generated
zap routes --json
zap routes --verbose
```

**Options:**
- `-d, --routes-dir <dir>` - Routes directory path
- `-o, --output <dir>` - Output directory for generated files
- `--json` - Output routes as JSON
- `--verbose` - Show full handler code

## Project Structure

A typical ZapJS project structure:

```
my-app/
├── routes/                    # File-based routing
│   ├── __root.tsx            # Root layout
│   ├── index.tsx             # Home page (/)
│   ├── about.tsx             # About page (/about)
│   ├── posts/
│   │   ├── [id].tsx         # Dynamic route (/posts/:id)
│   │   └── index.tsx        # Posts index (/posts)
│   └── api/
│       ├── hello.ts         # API route (/api/hello)
│       └── users.$id.ts     # Dynamic API route (/api/users/:id)
├── server/
│   ├── src/
│   │   └── main.rs          # Rust server entry point
│   └── Cargo.toml           # Rust dependencies
├── src/
│   └── generated/           # Auto-generated files
│       ├── routes.tsx       # Generated route tree
│       └── routeManifest.json
├── package.json
├── Cargo.toml              # Workspace Cargo.toml
├── tsconfig.json
└── zap.config.ts           # Optional config
```

## Configuration

### `zap.config.ts` (Optional)

```typescript
import { defineConfig } from 'zap';

export default defineConfig({
  server: {
    port: 3000,
    hostname: '127.0.0.1',
  },
  dev: {
    apiPort: 3000,
    clientPort: 5173,
    watchRust: true,
    watchTypeScript: true,
    open: true,
  },
});
```

## Routing Conventions

### File-Based Routing

ZapJS uses Next.js-style file-based routing:

- `routes/index.tsx` → `/`
- `routes/about.tsx` → `/about`
- `routes/posts/[id].tsx` → `/posts/:id`
- `routes/posts/[...slug].tsx` → `/posts/*` (catch-all)
- `routes/__root.tsx` → Root layout wrapper
- `routes/_layout.tsx` → Layout wrapper

### API Routes

TypeScript files in `routes/api/` become API endpoints:

```typescript
// routes/api/users.ts
export const GET = async () => {
  return { users: [] };
};

export const POST = async ({ request }: { request: Request }) => {
  const body = await request.json();
  return { created: body };
};
```

**Supported Methods:** `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`

### Dynamic Routes

Use `[param]` or `$param` syntax for dynamic segments:

```typescript
// routes/api/users.$id.ts or routes/api/users/[id].ts
export const GET = async ({ params }: { params: { id: string } }) => {
  return { id: params.id };
};
```

## Troubleshooting

### `zap` command not found

After installing globally, if `zap` is not found:

1. Check npm global bin directory is in PATH:
   ```bash
   npm config get prefix
   ```

2. Add npm global bin to your PATH:
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   export PATH="$(npm config get prefix)/bin:$PATH"
   ```

3. Verify installation:
   ```bash
   which zap
   zap --version
   ```

### Development server won't start

1. Ensure Rust is installed:
   ```bash
   rustc --version
   cargo --version
   ```

2. Clean and rebuild:
   ```bash
   cargo clean
   npm run build
   zap dev
   ```

3. Check port availability:
   ```bash
   lsof -i :3000  # Check if port 3000 is in use
   ```

### TypeScript bindings not generating

1. Ensure `zap-codegen` binary exists:
   ```bash
   cargo build --release --bin zap-codegen
   ```

2. Manually run codegen:
   ```bash
   zap codegen
   ```

## Links

- [Documentation](https://github.com/yourusername/zapjs)
- [Examples](https://github.com/yourusername/zapjs/tree/main/examples)
- [Discord Community](https://discord.gg/zapjs)
- [GitHub Issues](https://github.com/yourusername/zapjs/issues)

## License

MIT
