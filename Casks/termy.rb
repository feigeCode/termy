cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.49"
  sha256 arm:   "5df0d96d265e556bee31a32aec71d4ece0aa55609905579e0da56e52a0f9b7f8",
         intel: "cb192a292cf6020c2e00baa00657267e8be9612caf930c4d15f23e576aebb46d"

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
