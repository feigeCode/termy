cask "termy" do
  arch arm: "arm64", intel: "x86_64"

  version "0.1.31"
  sha256 arm:   "fb19579ddabad7a475442ed5b93799f14ab354f84b6c93ebc92e6029ab91e4a0",
         intel: "d39556e38b148841d15445062ab0cd0055b84f1c264d067afdb5d2a7623f7117"

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
