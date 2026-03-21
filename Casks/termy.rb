cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.64"
  sha256 arm:   "06a0347a34254b030a73528f2362c1fc5e246aafb1ac06d9b57e678a90a53126",
         intel: "1801170d934295a9eadcc86187d680aee429ab459a5898f05b81166650b7e15d"

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
