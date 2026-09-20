cask "local-connector" do
  version "0.4.3"
  sha256 "d08988ca8ea13272890defc454f5c4f786b51470b3bb27389e3b05fa48b0253a"

  url "https://github.com/whzxc/chatgpt-local-connector/releases/download/v#{version}/Local.Connector_#{version}_aarch64.dmg"
  name "Local Connector"
  desc "Connect ChatGPT to local Codex Desktop through Secure MCP Tunnel"
  homepage "https://github.com/whzxc/chatgpt-local-connector"
  depends_on arch: :arm64
  auto_updates true
  app "Local Connector.app"

  caveats <<~EOS
    This application is not Developer ID signed or notarized.
    See the repository installation guide if macOS blocks the first launch.
    Codex Desktop, Tunnel credentials and ChatGPT plugin setup are required.
  EOS
end
