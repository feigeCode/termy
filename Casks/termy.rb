cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.54"
  sha256 arm:   "fe72d476d93fe08210d363437c09b46363665559ba5385e7fc8217f78c0f7b22",
         intel: "d2a27df387f28bae9eaba0244ef92492cf4cc981add0a84c1db1a3ace219f566"

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
