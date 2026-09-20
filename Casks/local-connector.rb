cask "local-connector" do
  version "0.4.0"
  sha256 "ee5bef5d073bc0aaf685feb2bf2a99c646fdab5c263cc100c7d109057898be51"

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
