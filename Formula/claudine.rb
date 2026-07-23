class Claudine < Formula
  desc "Outil Rust TUI/CLI pour naviguer et gérer les données locales de Claude Code (~/.claude)"
  homepage "https://github.com/systm-d/claudine"
  url "https://github.com/systm-d/claudine/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "6528becc578ae826ab1cc1ca9e1a96b953aeb1bdbd4badd4e4be04b3fa1bb0b9"
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
