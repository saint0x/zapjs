#!/bin/bash
# Install benchmarking tools
# Supports macOS and Linux

set -e

echo "🔧 Installing benchmarking tools..."

# Detect OS
OS="$(uname -s)"

case "${OS}" in
    Linux*)
        echo "📦 Detected Linux"

        # Check if running as root for apt
        if [ "$EUID" -ne 0 ]; then
            echo "⚠️  Please run with sudo for Linux installation"
            exit 1
        fi

        # Install wrk
        if ! command -v wrk &> /dev/null; then
            echo "📥 Installing wrk..."
            apt-get update
            apt-get install -y wrk
        else
            echo "✅ wrk already installed"
        fi

        # Install Rust (for Criterion benchmarks)
        if ! command -v cargo &> /dev/null; then
            echo "📥 Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        else
            echo "✅ Rust already installed"
        fi

        # Install Bun (for TypeScript benchmarks)
        if ! command -v bun &> /dev/null; then
            echo "📥 Installing Bun..."
            curl -fsSL https://bun.sh/install | bash
        else
            echo "✅ Bun already installed"
        fi

        echo "✅ All tools installed on Linux"
        ;;

    Darwin*)
        echo "📦 Detected macOS"

        # Check for Homebrew
        if ! command -v brew &> /dev/null; then
            echo "❌ Homebrew not found. Please install from https://brew.sh"
            exit 1
        fi

        # Install wrk
        if ! command -v wrk &> /dev/null; then
            echo "📥 Installing wrk..."
            brew install wrk
        else
            echo "✅ wrk already installed"
        fi

        # Install Rust
        if ! command -v cargo &> /dev/null; then
            echo "📥 Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        else
            echo "✅ Rust already installed"
        fi

        # Install Bun
        if ! command -v bun &> /dev/null; then
            echo "📥 Installing Bun..."
            brew install oven-sh/bun/bun
        else
            echo "✅ Bun already installed"
        fi

        echo "✅ All tools installed on macOS"
        ;;

    *)
        echo "❌ Unsupported OS: ${OS}"
        exit 1
        ;;
esac

echo ""
echo "📊 Installed tools:"
echo "  wrk:  $(wrk --version 2>&1 | head -1)"
echo "  cargo: $(cargo --version)"
echo "  bun:  $(bun --version)"
echo ""
echo "✅ Setup complete! You can now run benchmarks."
