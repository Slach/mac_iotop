#!/bin/bash
# Updates Homebrew formula in Slach/homebrew-tap with new version and checksums.
# Usage: update-formula.sh <version> <sha256-arm64> <sha256-amd64>
set -euo pipefail

VERSION="$1"
SHA_ARM64="$2"
SHA_AMD64="$3"

cat > /tmp/mac_iotop.rb <<'EOF'
class MacIotop < Formula
  desc "Simple macOS disk I/O monitor — like iotop for macOS"
  homepage "https://github.com/Slach/mac_iotop"
  version "VERSION_PLACEHOLDER"
  license "MIT"

  on_arm do
    url "https://github.com/Slach/mac_iotop/releases/download/v#{version}/mac_iotop-arm64"
    sha256 "SHA_ARM64_PLACEHOLDER"
  end

  on_intel do
    url "https://github.com/Slach/mac_iotop/releases/download/v#{version}/mac_iotop-amd64"
    sha256 "SHA_AMD64_PLACEHOLDER"
  end

  def install
    binary = Hardware::CPU.arm? ? "mac_iotop-arm64" : "mac_iotop-amd64"
    bin.install binary => "mac_iotop"
  end

  test do
    assert_match "mac_iotop", shell_output("#{bin}/mac_iotop --help 2>&1", 1)
  end
end
EOF

sed -i "s/VERSION_PLACEHOLDER/${VERSION}/" /tmp/mac_iotop.rb
sed -i "s/SHA_ARM64_PLACEHOLDER/${SHA_ARM64}/" /tmp/mac_iotop.rb
sed -i "s/SHA_AMD64_PLACEHOLDER/${SHA_AMD64}/" /tmp/mac_iotop.rb

CONTENT=$(base64 -w 0 /tmp/mac_iotop.rb)
EXISTING_SHA=$(gh api repos/Slach/homebrew-tap/contents/Formula/mac_iotop.rb --jq .sha)

gh api repos/Slach/homebrew-tap/contents/Formula/mac_iotop.rb \
  --method PUT \
  --field message="Update mac_iotop to ${VERSION}" \
  --field content="${CONTENT}" \
  --field sha="${EXISTING_SHA}"
