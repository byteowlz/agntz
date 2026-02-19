class Agntz < Formula
  desc "Agent utility toolkit for AI coding agents"
  homepage "https://github.com/byteowlz/agntz"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/byteowlz/agntz/releases/download/v#{version}/agntz-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "DARWIN_X86_SHA256"
    end
    on_arm do
      url "https://github.com/byteowlz/agntz/releases/download/v#{version}/agntz-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "DARWIN_ARM_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/byteowlz/agntz/releases/download/v#{version}/agntz-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "LINUX_X86_SHA256"
    end
    on_arm do
      url "https://github.com/byteowlz/agntz/releases/download/v#{version}/agntz-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "LINUX_ARM_SHA256"
    end
  end

  def install
    bin.install "agntz"
  end

  test do
    system "#{bin}/agntz", "--version"
  end
end
