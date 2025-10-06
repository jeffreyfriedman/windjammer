class Windjammer < Formula
  desc "A simple language that transpiles to Rust"
  homepage "https://github.com/jeffreyfriedman/windjammer"
  url "https://github.com/jeffreyfriedman/windjammer/archive/v0.7.0.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"  # Update on release
  license "MIT OR Apache-2.0"
  head "https://github.com/jeffreyfriedman/windjammer.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."
    
    # Install standard library
    (lib/"windjammer/std").install Dir["std/*"]
    
    # Set up shell completions (future)
    # generate_completions_from_executable(bin/"windjammer", "completions")
  end

  def caveats
    <<~EOS
      The Windjammer standard library is installed at:
        #{lib}/windjammer/std

      You may want to set the following environment variable:
        export WINDJAMMER_STDLIB=#{lib}/windjammer/std

      To get started:
        windjammer --help
        windjammer build --path <your_project>
    EOS
  end

  test do
    # Test that the binary runs
    assert_match "windjammer", shell_output("#{bin}/windjammer --version")
    
    # Test basic compilation
    (testpath/"test.wj").write <<~EOS
      fn main() {
          println!("Hello from Windjammer!")
      }
    EOS
    
    system bin/"windjammer", "build", "--path", testpath, "--output", testpath/"build", "--target", "wasm"
    assert_predicate testpath/"build/main.rs", :exist?
  end
end
