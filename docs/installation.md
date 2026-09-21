# Installation and updates

**English** | [简体中文](zh-CN/installation.md)

## Download

Download the installer and `SHA256SUMS.txt` from [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases/latest). The release page is the source of truth for available versions and files.

| Platform | File | Support |
| --- | --- | --- |
| macOS | `Local.Connector_<version>_aarch64.dmg` | Apple Silicon only |
| Windows x64 | `…-setup.exe` / `….msi` | Requires Microsoft Store Codex Desktop |

You do not need Node, npm, Rust, or Cargo to run the app. Install Codex Desktop and sign in before use. Official Tunnel mode requires a Tunnel ID and runtime API Key; the app prepares connection components on first connect. HTTPS MCP supports a temporary Cloudflare trial without an account, a fixed domain, ngrok, or your reverse proxy, with required client components managed by the app. See the [connection guide](tunnel.md). Both modes require adding a connection in ChatGPT before remote use; see [Codex-assisted setup](codex-setup.md).

## macOS installation

Open the DMG, drag Local Connector into Applications, eject the image, and launch from Applications. Do not keep running it from the DMG. Before replacing an installation, quit Local Connector from the menu bar and drag in the new app. Configuration and Codex history are stored outside the app and survive replacement.

The project does not currently use Apple Developer ID signing or notarization. macOS may block the first launch. After confirming the download source and SHA256, choose **Open Anyway** in System Settings → Privacy & Security. If macOS reports that the app is damaged, remove the quarantine flag only from this verified app:

```sh
xattr -rd com.apple.quarantine "/Applications/Local Connector.app"
```

If permission is denied, use an administrator account to handle this app's permissions. You do not need to disable Gatekeeper globally. If the hash differs, download again instead of removing quarantine.

Verify a download:

```sh
shasum -a 256 Local.Connector_0.6.1_aarch64.dmg
```

Compare the result with the corresponding entry in `SHA256SUMS.txt` for the same release.

## Homebrew

Install using the repository's Cask:

```sh
brew tap whzxc/chatgpt-local-connector https://github.com/whzxc/chatgpt-local-connector
brew install --cask local-connector
```

If quarantine blocks launch, after verifying the source you can use `brew install --cask --no-quarantine local-connector`. To replace a manually installed app with the same name, quit it and use `brew install --cask --force local-connector`. The Cask does not automatically run sudo or clear all extended attributes from the app.

In-app updates work directly. To update through Homebrew:

```sh
brew update
brew upgrade --cask --greedy local-connector
```

## Windows installation

Run the x64 NSIS `.exe` or MSI installer. Install and sign in to Microsoft Store Codex Desktop. Connector connects to Desktop for the same user over named pipes; tasks open and run in Desktop. Installers are not Authenticode-signed, so Windows may show an unknown publisher. An administrator must handle enterprise policies that block execution; the app cannot bypass them.

PowerShell verification example:

```powershell
Get-FileHash .\Local.Connector_0.6.1_x64-setup.exe -Algorithm SHA256
```

To upgrade, quit the app and install over it using the same installer type. Configuration is preserved by default; uninstalling first is unnecessary.

## In-app updates

By default, the app checks every six hours while visible and shows an entry when a new version is available. In Settings you can disable automatic checks, check manually, read release notes, or skip a version. A manual check shows skipped versions again.

After selecting Download and install, progress is displayed. Downloads can be cancelled; installation cannot. The app downloads and verifies the update signature before closing the Connector connection, replacing and restarting the app, and restoring its previous connection state. It does not restart Codex Desktop or take ownership of its tasks.

Failures display an error with retry or manual download options. If installation fails, the app tries to restore the connection; any restoration failure is also reported. Update checks may fail when offline, when GitHub is inaccessible, or when no release is available; connection features remain usable.

Updates are signed with the project's independent key and verified with a public key embedded in the client. This does not replace Apple or Windows code signing.

## Removal

Disable connecting at sign-in in Settings, quit the app from the menu bar, then delete the app or use the system uninstaller. Homebrew users can run `brew uninstall --cask local-connector`.

Connector configuration remains in `~/.local/state/chatgpt-local-connector` on macOS or `%LOCALAPPDATA%/chatgpt-local-connector` on Windows by default. Delete that directory yourself only if you no longer need Tunnel settings, receipts, or logs. Codex manages its own projects and history.
