cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.57"
  sha256 arm:   "432a5d36e48dbe768e2bb5c4370df7ea91efe5ead4677c304d9a753212cdc9d9",
         intel: "d76dda4de9244d154a850d62ed381fd5ab1a4a613fc75552b4dd817601c44d07"

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
