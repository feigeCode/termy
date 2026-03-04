cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.38"
  sha256 arm:   "823569c08b5ca5670a47d5ab3e1f796bd06b311117fb7fada584e55761cd12c0",
         intel: "a59cb51ef015ae8aa186e795edb86529c8f914b5add98a4e32e2efc9ecb6755f"

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
