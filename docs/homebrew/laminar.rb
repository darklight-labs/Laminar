class Laminar < Formula
  desc "Dual-mode Zcash batch constructor and QR/UR generator"
  homepage "https://github.com/darklightlabs/laminar"
  url "https://github.com/darklightlabs/laminar/releases/download/v0.1.0/laminar-cli-x86_64-apple-darwin.tar.gz"
  version "0.1.0"
  sha256 "REPLACE_WITH_REAL_SHA256"
  license "MIT OR Apache-2.0"

  def install
    bin.install "laminar-cli"
    bin.install_symlink "laminar-cli" => "laminar"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/laminar --version")
  end
end
