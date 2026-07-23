class Claudine < Formula
  desc "Outil Rust TUI/CLI pour naviguer et gérer les données locales de Claude Code (~/.claude)"
  homepage "https://github.com/systm-d/claudine"
  url "https://github.com/systm-d/claudine/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "ce55eac3e2f35835673d81d841898b88e013de0042ba3978136a3b757b1e89b2"
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
