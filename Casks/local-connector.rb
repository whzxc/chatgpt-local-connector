cask "local-connector" do
  version "0.3.0"
  sha256 "8dff9bede33c71b9e569cf18c5b393df4db84bcae7c40448a8cf2a5bd91f9cd8"

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
