class HashFolderoo < Formula
  desc "Multi-command cross-platform folder hashing and diff toolkit"
  homepage "https://github.com/supermarsx/hash-folderoo"
  version "vSHORT"
  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/supermarsx/hash-folderoo/releases/download/vSHORT/hash-folderoo-aarch64-apple-darwin-vSHORT.tar.gz"
      sha256 "SHA256_PLACEHOLDER_MACOS_AARCH64"
    else
      url "https://github.com/supermarsx/hash-folderoo/releases/download/vSHORT/hash-folderoo-x86_64-apple-darwin-vSHORT.tar.gz"
      sha256 "SHA256_PLACEHOLDER_MACOS_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/supermarsx/hash-folderoo/releases/download/vSHORT/hash-folderoo-aarch64-unknown-linux-gnu-vSHORT.tar.gz"
      sha256 "SHA256_PLACEHOLDER_LINUX_AARCH64"
    else
      url "https://github.com/supermarsx/hash-folderoo/releases/download/vSHORT/hash-folderoo-x86_64-unknown-linux-gnu-vSHORT.tar.gz"
      sha256 "SHA256_PLACEHOLDER_LINUX_X86_64"
    end
  end

  def install
    bin.install "hash-folderoo"
  end

  test do
    system "#{bin}/hash-folderoo", "--version"
  end
end
