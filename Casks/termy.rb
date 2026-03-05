cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.42"
  sha256 arm:   "b1a5f7da06e446aa11c87f6ea43b18780280a582e8d0a7dbee680d26db7d6731",
         intel: "5a750e7df4f1f6d2e706f59d89154d04417b5eb0b85fea7e133b32e3cd4d3f3c"

  url "https://github.com/lassejlv/termy/releases/download/v#{version}/Termy-v#{version}-macos-#{arch}.dmg"
  name "Termy"
  desc "Minimal GPU-powered terminal written in Rust"
  homepage "https://github.com/lassejlv/termy"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :big_sur"

  app "Termy.app"
end
