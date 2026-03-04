cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.40"
  sha256 arm:   "1a4df69acbdd6bc357b9e03ec18523a66f67d57cf9b2115be3bf48287d32468f",
         intel: "4b80be6d84787c82ba493a76e1e74d545907e839c06104bf6e387bbb8a1b5ef5"

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
