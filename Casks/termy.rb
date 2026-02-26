cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.32"
  sha256 arm:   "4679f4b812e4aea4e6f15affb48be615a50a06c198bad6938397b6b1dae34d52",
         intel: "bbce461e1775eb8d4321e1e4a3e7a2cc15950097fb77d3684dd00389196f67b6"

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
