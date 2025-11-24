# Homebrew packaging config

This folder contains a sample Homebrew formula template (`hash-folderoo.rb`) that can be published in a Homebrew Tap. During automated release we populate the following placeholders with the release tag and checksums:

- `vSHORT` - release tag/version (for example `v1a2b3c4`)
- `SHA256_PLACEHOLDER_MACOS_X86_64` - checksum for macOS x86_64 artifact
- `SHA256_PLACEHOLDER_MACOS_AARCH64` - checksum for macOS arm64 artifact
- `SHA256_PLACEHOLDER_LINUX_X86_64` - checksum for linux x86_64 artifact
- `SHA256_PLACEHOLDER_LINUX_AARCH64` - checksum for linux arm64 artifact

Suggested tap name: `supermarsx/homebrew-tap` with the formula path `Formula/hash-folderoo.rb`.

Homebrew will prefer bottles (prebuilt binaries) if provided by the tap; the formula above points directly at GitHub release tarballs as a simple distribution pattern.
