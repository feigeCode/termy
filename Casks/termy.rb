cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.51"
  sha256 arm:   "f0007ff231efb3530b1b39cc346b2b897cd6905d48916f0f3b0fd0ec8bd31c12",
         intel: "7853967938f719eff9d1d64bfafdcb9730f9f8dd28b2c972ce36fe245a4caf4d"

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
