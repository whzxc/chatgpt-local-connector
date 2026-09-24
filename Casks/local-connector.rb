cask "local-connector" do
  version "0.8.2"
  sha256 "e3d2936f7499fa3f807d9ec17d95ebcdb2defe1c471b3f8adcd97dedbea8dcc3"

  url "https://github.com/whzxc/chatgpt-local-connector/releases/download/v#{version}/Local.Connector_#{version}_aarch64.dmg"
  name "Local Connector"
  desc "Connect ChatGPT to local Codex Desktop"
  homepage "https://github.com/whzxc/chatgpt-local-connector"
  depends_on arch: :arm64
  auto_updates true
  app "Local Connector.app"

  caveats <<~EOS
    This application is not Developer ID signed or notarized.
    See the repository installation guide if macOS blocks the first launch.
    Codex Desktop and a ChatGPT connection are required.
    Choose OpenAI Secure MCP Tunnel or HTTPS MCP in the application.
  EOS
end
