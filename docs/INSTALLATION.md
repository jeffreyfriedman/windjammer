# Windjammer Installation Guide

This guide covers all available methods to install Windjammer on your system.

---

## Quick Start

**Recommended for most users:**

### macOS / Linux
```bash
# Using Homebrew (easiest)
brew tap jeffreyfriedman/windjammer
brew install windjammer

# Or using cargo
cargo install windjammer
```

### Windows
```powershell
# Using Scoop
scoop bucket add windjammer https://github.com/jeffreyfriedman/scoop-windjammer
scoop install windjammer

# Or using cargo
cargo install windjammer
```

---

## Installation Methods

### 1. Cargo (All Platforms)

**Prerequisites:** [Rust](https://rustup.rs/) installed

```bash
cargo install windjammer
```

**Pros:**
- ✅ Works on all platforms
- ✅ Always up-to-date
- ✅ Easy to uninstall (`cargo uninstall windjammer`)

**Cons:**
- ⚠️ Requires Rust toolchain
- ⚠️ Compiles from source (slower)

---

### 2. Homebrew (macOS / Linux)

```bash
# Add the Windjammer tap
brew tap jeffreyfriedman/windjammer

# Install Windjammer
brew install windjammer

# Verify installation
windjammer --version
```

**Update:**
```bash
brew update
brew upgrade windjammer
```

**Uninstall:**
```bash
brew uninstall windjammer
brew untap jeffreyfriedman/windjammer
```

---

### 3. Pre-built Binaries

Download pre-compiled binaries from [GitHub Releases](https://github.com/jeffreyfriedman/windjammer/releases).

#### Linux (x86_64)
```bash
# Download
curl -LO https://github.com/jeffreyfriedman/windjammer/releases/latest/download/windjammer-linux-x86_64.tar.gz

# Extract
tar -xzf windjammer-linux-x86_64.tar.gz

# Install
sudo mv windjammer /usr/local/bin/
sudo chmod +x /usr/local/bin/windjammer

# Verify
windjammer --version
```

#### Linux (aarch64 / ARM)
```bash
curl -LO https://github.com/jeffreyfriedman/windjammer/releases/latest/download/windjammer-linux-aarch64.tar.gz
tar -xzf windjammer-linux-aarch64.tar.gz
sudo mv windjammer /usr/local/bin/
sudo chmod +x /usr/local/bin/windjammer
```

#### macOS (Intel)
```bash
curl -LO https://github.com/jeffreyfriedman/windjammer/releases/latest/download/windjammer-macos-x86_64.tar.gz
tar -xzf windjammer-macos-x86_64.tar.gz
sudo mv windjammer /usr/local/bin/
sudo chmod +x /usr/local/bin/windjammer
```

#### macOS (Apple Silicon)
```bash
curl -LO https://github.com/jeffreyfriedman/windjammer/releases/latest/download/windjammer-macos-aarch64.tar.gz
tar -xzf windjammer-macos-aarch64.tar.gz
sudo mv windjammer /usr/local/bin/
sudo chmod +x /usr/local/bin/windjammer
```

#### Windows
1. Download `windjammer-windows-x86_64.zip` from [Releases](https://github.com/jeffreyfriedman/windjammer/releases)
2. Extract the ZIP file
3. Move `windjammer.exe` to a directory in your PATH
4. Open Command Prompt and verify: `windjammer --version`

---

### 4. Build from Source

**Prerequisites:**
- Git
- [Rust](https://rustup.rs/) (latest stable)

```bash
# Clone the repository
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer

# Run the installation script
./install.sh
```

**Manual build:**
```bash
# Clone and build
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer
cargo build --release

# Install manually
sudo cp target/release/windjammer /usr/local/bin/
sudo mkdir -p /usr/local/lib/windjammer/std
sudo cp -r std/* /usr/local/lib/windjammer/std/
```

---

### 5. Docker

```bash
# Pull the latest image
docker pull ghcr.io/jeffreyfriedman/windjammer:latest

# Run Windjammer
docker run --rm ghcr.io/jeffreyfriedman/windjammer:latest --version

# Build a project (mount your code)
docker run --rm -v $(pwd):/workspace ghcr.io/jeffreyfriedman/windjammer:latest \
  build --path /workspace --output /workspace/build --target wasm
```

**Create an alias for convenience:**
```bash
# Add to ~/.bashrc or ~/.zshrc
alias windjammer='docker run --rm -v $(pwd):/workspace ghcr.io/jeffreyfriedman/windjammer:latest'

# Usage
windjammer --help
windjammer build --path . --output ./build --target wasm
```

**Pros:**
- ✅ No local Rust installation needed
- ✅ Isolated environment
- ✅ Consistent across platforms

**Cons:**
- ⚠️ Slower (container overhead)
- ⚠️ Requires Docker

---

### 6. Snap (Linux)

```bash
# Install
sudo snap install windjammer --classic

# Verify
windjammer --version
```

**Update:**
```bash
sudo snap refresh windjammer
```

**Uninstall:**
```bash
sudo snap remove windjammer
```

---

### 7. Scoop (Windows)

```powershell
# Add the Windjammer bucket
scoop bucket add windjammer https://github.com/jeffreyfriedman/scoop-windjammer

# Install
scoop install windjammer

# Verify
windjammer --version
```

**Update:**
```powershell
scoop update
scoop update windjammer
```

**Uninstall:**
```powershell
scoop uninstall windjammer
```

---

### 8. APT (Debian / Ubuntu)

```bash
# Add the repository
echo "deb https://github.com/jeffreyfriedman/windjammer/releases/download/apt /" | \
  sudo tee /etc/apt/sources.list.d/windjammer.list

# Install
sudo apt update
sudo apt install windjammer

# Verify
windjammer --version
```

**Update:**
```bash
sudo apt update
sudo apt upgrade windjammer
```

**Uninstall:**
```bash
sudo apt remove windjammer
```

---

## Verify Installation

After installation, verify Windjammer is working:

```bash
# Check version
windjammer --version

# View help
windjammer --help

# Try compiling an example
echo 'fn main() { println!("Hello, Windjammer!") }' > test.wj
windjammer build --path . --output ./build --target wasm
```

---

## Configuration

### Standard Library Location

Windjammer looks for its standard library in these locations (in order):

1. `$WINDJAMMER_STDLIB` environment variable
2. `./std/` (relative to current directory)
3. `/usr/local/lib/windjammer/std` (Unix)
4. `C:\Program Files\Windjammer\std` (Windows)

**Set custom location:**

```bash
# Add to ~/.bashrc, ~/.zshrc, or ~/.profile
export WINDJAMMER_STDLIB=/path/to/windjammer/std
```

```powershell
# Windows (PowerShell)
[Environment]::SetEnvironmentVariable("WINDJAMMER_STDLIB", "C:\path\to\std", "User")
```

---

## Troubleshooting

### Command not found

**Issue:** `windjammer: command not found`

**Solution:**
- Ensure the installation directory is in your `PATH`
- Restart your terminal or run `source ~/.bashrc` (or equivalent)

```bash
# Check if windjammer is in PATH
which windjammer

# Add to PATH if needed (add to ~/.bashrc or ~/.zshrc)
export PATH="/usr/local/bin:$PATH"
```

### Permission denied (Unix)

**Issue:** Permission errors during installation

**Solution:**
```bash
# Use sudo for system-wide installation
sudo cp windjammer /usr/local/bin/

# Or install to user directory
mkdir -p ~/.local/bin
cp windjammer ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"
```

### Standard library not found

**Issue:** `Error: Cannot find standard library`

**Solution:**
```bash
# Set WINDJAMMER_STDLIB environment variable
export WINDJAMMER_STDLIB=/path/to/windjammer/std

# Or copy stdlib to default location
sudo mkdir -p /usr/local/lib/windjammer
sudo cp -r std /usr/local/lib/windjammer/
```

### Rust version too old

**Issue:** Compilation errors due to old Rust version

**Solution:**
```bash
# Update Rust
rustup update stable
rustup default stable
```

### Windows: Security warning

**Issue:** Windows SmartScreen blocks execution

**Solution:**
1. Right-click `windjammer.exe`
2. Select "Properties"
3. Check "Unblock" at the bottom
4. Click "OK"

---

## Uninstallation

### Cargo
```bash
cargo uninstall windjammer
```

### Homebrew
```bash
brew uninstall windjammer
brew untap jeffreyfriedman/windjammer
```

### Manual (Unix)
```bash
sudo rm /usr/local/bin/windjammer
sudo rm -rf /usr/local/lib/windjammer
```

### Manual (Windows)
1. Delete `windjammer.exe` from your PATH directory
2. Remove `C:\Program Files\Windjammer` (if exists)
3. Remove `WINDJAMMER_STDLIB` environment variable

---

## Next Steps

After installation:

1. **Read the Guide**: [`docs/GUIDE.md`](GUIDE.md)
2. **Try Examples**: [`examples/`](../examples/)
3. **Explore Stdlib**: [`docs/MODULE_SYSTEM.md`](MODULE_SYSTEM.md)
4. **Build a Project**: Start with `windjammer build --help`

---

## Getting Help

- **Documentation**: [docs/](.)
- **Issues**: [GitHub Issues](https://github.com/jeffreyfriedman/windjammer/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jeffreyfriedman/windjammer/discussions)

---

**Happy coding with Windjammer!** 🚀
