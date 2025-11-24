# Homebrew packaging config

This folder contains a sample Homebrew formula template (`hash-folderoo.rb`) that can be published in a Homebrew Tap. Replace `vSHORT` and `SHA256_PLACEHOLDER` with the release version and checksum produced by CI.

Suggested tap name: `supermarsx/homebrew-tap` with the formula path `Formula/hash-folderoo.rb`.

Homebrew will prefer bottles (prebuilt binaries) if provided by the tap; the formula above points directly at GitHub release tarballs as a simple distribution pattern.
