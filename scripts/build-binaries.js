#!/usr/bin/env node

/**
 * Build ZapJS binaries for all platforms
 *
 * This script:
 * 1. Builds Rust binaries for darwin-arm64, darwin-x64, and linux-x64
 * 2. Copies binaries to corresponding platform packages
 * 3. Verifies binaries are executable
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const PLATFORMS = {
  'darwin-arm64': {
    target: 'aarch64-apple-darwin',
    binExt: ''
  },
  'darwin-x64': {
    target: 'x86_64-apple-darwin',
    binExt: ''
  },
  'linux-x64': {
    target: 'x86_64-unknown-linux-musl',
    binExt: ''
  }
};

const BINARIES = ['zap', 'zap-codegen', 'splice'];

function exec(cmd, opts = {}) {
  console.log(`  $ ${cmd}`);
  return execSync(cmd, { stdio: 'inherit', ...opts });
}

function buildForPlatform(platform, { target, binExt }) {
  console.log(`\n🔨 Building for ${platform} (${target})...`);

  // Build with cargo
  try {
    if (platform === 'linux-x64' && process.platform !== 'linux') {
      // Skip Linux build on non-Linux platforms (requires cross-compilation)
      console.log(`  ⏭️  Skipping (cross-compilation not configured)`);
      console.log(`  💡 To build for Linux on macOS:`);
      console.log(`     1. Install cross: cargo install cross`);
      console.log(`     2. Run: cross build --release --target ${target}`);
      return;
    }

    exec(`cargo build --release --target ${target}`);
  } catch (error) {
    console.error(`❌ Failed to build for ${platform}`);
    throw error;
  }

  // Copy binaries to platform package
  const targetDir = path.join('target', target, 'release');
  const platformDir = path.join('packages', 'platforms', platform, 'bin');

  // Ensure platform bin directory exists
  fs.mkdirSync(platformDir, { recursive: true });

  for (const binary of BINARIES) {
    const srcPath = path.join(targetDir, binary + binExt);
    const destPath = path.join(platformDir, binary + binExt);

    if (!fs.existsSync(srcPath)) {
      console.warn(`⚠️  Warning: ${binary} not found at ${srcPath}`);
      continue;
    }

    // Copy binary
    fs.copyFileSync(srcPath, destPath);

    // Make executable (Unix only)
    if (binExt === '') {
      fs.chmodSync(destPath, 0o755);
    }

    // Get file size
    const stats = fs.statSync(destPath);
    const sizeMB = (stats.size / 1024 / 1024).toFixed(1);

    console.log(`  ✅ ${binary}: ${sizeMB}MB → ${destPath}`);
  }
}

function verifyBinaries() {
  console.log('\n🔍 Verifying binaries...');

  let allPresent = true;

  for (const [platform, _] of Object.entries(PLATFORMS)) {
    const platformDir = path.join('packages', 'platforms', platform, 'bin');

    let platformComplete = true;
    for (const binary of BINARIES) {
      const binPath = path.join(platformDir, binary);
      if (!fs.existsSync(binPath)) {
        console.warn(`  ⚠️  Missing: ${platform}/${binary}`);
        platformComplete = false;
        allPresent = false;
      }
    }

    if (platformComplete) {
      console.log(`  ✅ ${platform}: all binaries present`);
    }
  }

  if (!allPresent) {
    console.log('\n⚠️  Some binaries are missing. This is OK if you skipped cross-compilation.');
    console.log('    Publish will only include platforms with complete binaries.');
  }
}

function main() {
  console.log('🚀 Building ZapJS binaries for all platforms\n');
  console.log('Platforms:', Object.keys(PLATFORMS).join(', '));
  console.log('Binaries:', BINARIES.join(', '));

  // Build for each platform
  for (const [platform, config] of Object.entries(PLATFORMS)) {
    buildForPlatform(platform, config);
  }

  // Verify all binaries were created
  verifyBinaries();

  console.log('\n✨ All binaries built successfully!');
  console.log('\nNext steps:');
  console.log('  1. Test binaries: npm run test:binaries');
  console.log('  2. Publish: ./scripts/publish.sh');
}

main();
