class Claudine < Formula
  desc "Outil Rust TUI/CLI pour naviguer et gérer les données locales de Claude Code (~/.claude)"
  homepage "https://github.com/systm-d/claudine"
  url "https://github.com/systm-d/claudine/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "39b6fae14f787f34116d1f809c7b3e52a57dcffff51f03d7a05a548e520e1da5"
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
