class Claudine < Formula
  desc "Outil Rust TUI/CLI pour naviguer et gérer les données locales de Claude Code (~/.claude)"
  homepage "https://github.com/systm-d/claudine"
  url "https://github.com/systm-d/claudine/archive/refs/tags/v0.1.3.tar.gz"
  sha256 "802ed405986859ec9bacf1e19c533931b1c2da579c4a6315c6ca7a9f49c9f913"
  license "MIT OR Apache-2.0"
  head "https://github.com/systm-d/claudine.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/claudine")
  end

  test do
    system bin/"claudine", "--version"
  end
end
