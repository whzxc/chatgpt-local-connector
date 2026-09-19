cask "local-connector" do
  version "0.3.0"
  sha256 "0b740145995907470b39cb5372518601d7f612fc5e7439ea1b711179db2e6747"

  url "https://github.com/whzxc/chatgpt-local-connector/releases/download/v#{version}/Local.Connector_#{version}_universal.dmg"
  name "Local Connector"
  desc "Connect ChatGPT to local Codex Desktop through Secure MCP Tunnel"
  homepage "https://github.com/whzxc/chatgpt-local-connector"
  auto_updates true
  app "Local Connector.app"

  caveats <<~EOS
    This application is not Developer ID signed or notarized.
    See the repository installation guide if macOS blocks the first launch.
    Codex Desktop, Tunnel credentials and ChatGPT plugin setup are required.
  EOS
end
