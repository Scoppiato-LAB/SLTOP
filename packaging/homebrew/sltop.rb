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
      sha256 "5aa192b51cd5229a4bdfac6f8178af11b135e35ddb4d2f3d218adafce505c92a"
    else
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-darwin-amd64.tar.gz"
      sha256 "5f65d005a6644d22c8d5c9757483a66840bc6210c3270872050aaa2c3f24c17a"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-linux-arm64.tar.gz"
      sha256 "85013943fe73bc6d68905ad2cf7899e434b39bc10881b1f601f393e8399888c0"
    else
      url "https://github.com/Scoppiato-LAB/SLTOP/releases/download/v#{version}/sltop-linux-amd64.tar.gz"
      sha256 "e5e3cc375ae028690facbfc5270521b60f4028a3f9204a0b9db95c0586985abd"
    end
  end

  def install
    bin.install "sltop"
  end

  test do
    assert_match "sltop", shell_output("#{bin}/sltop --help")
  end
end
