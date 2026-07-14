class Claudine < Formula
  desc "Outil Rust TUI/CLI pour naviguer et gérer les données locales de Claude Code (~/.claude)"
  homepage "https://github.com/systm-d/claudine"
  url "https://github.com/systm-d/claudine/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "11479e7875775bd04e18831fcf9541486af8b580a2c52518392bedd133d06b1e"
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
