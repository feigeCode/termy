cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.63"
  sha256 arm:   "00ed5d17de5c9ec67f0ba65dee75619abbb7e66cb4b4f75671913528c37e9345",
         intel: "7dcc43382284390c0d845dc82b5e3c6b74007a43c20107537d6b9f3872ef0dcb"

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
