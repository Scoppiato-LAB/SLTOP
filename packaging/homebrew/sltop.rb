# typed: false
# frozen_string_literal: true

class Sltop < Formula
  desc "Lightweight real-time terminal system monitor by Scoppiato LAB"
  homepage "https://scoppiato.it"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-darwin-arm64.tar.gz"
      # sha256 "REPLACE_WITH_SHA256_FROM_CHECKSUMS_TXT"
    else
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-darwin-amd64.tar.gz"
      # sha256 "REPLACE_WITH_SHA256_FROM_CHECKSUMS_TXT"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-linux-arm64.tar.gz"
      # sha256 "REPLACE_WITH_SHA256_FROM_CHECKSUMS_TXT"
    else
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-linux-amd64.tar.gz"
      # sha256 "REPLACE_WITH_SHA256_FROM_CHECKSUMS_TXT"
    end
  end

  def install
    bin.install "sltop"
  end

  test do
    assert_match "sltop", shell_output("#{bin}/sltop --help")
  end
end
