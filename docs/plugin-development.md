# Plugin development and packaging

For installation and everyday use, see the [plugin guide](plugin.md).

## Development on macOS or Windows

Install Node 24.12+, npm and stable Rust, then run on the development device:

```sh
npm ci
npm run plugin:dev
```

This builds the native entrypoint and self-contained panel, starts a local Core lease, exports a repeatable `clc-dev` marketplace, and installs it with the host CLI. Keep this command running, then open the plugin in ChatGPT Desktop. Reload the host plugin after the initial installation; restart the host if it offers no plugin reload.

Saving React, CSS or shared UI source rebuilds the embedded HTML atomically. Visible development panels poll their own resource through the existing MCP channel and reload after a successful rebuild. A reload resets the panel's temporary selection and scroll position. A compilation failure keeps the last working panel; fix the error and save to retry. This is automatic full-panel reload, not React Fast Refresh. No HTTP server, certificate, browser security override or public tunnel is needed. The current desktop sandbox blocks loopback network requests even with HTTPS and a trusted certificate, so direct Vite HMR cannot run inside that host. The standalone desktop UI continues to use its ordinary Vite development server.

Rust, the development reload helper, manifest and skill changes require restarting this command and reloading the host plugin. They are not in-process hot replacement. Production panels always use embedded build resources and do not poll for source changes.

Development uses an isolated persistent data directory ending in `chatgpt-local-connector-dev` and a generated marketplace under the Codex home. `CLC_STATE_DIR` and `CLC_PLUGIN_DEV_DIR` override those locations. The development manifest points to an absolute debug executable and generated HTML file outside the plugin cache. To share Core with desktop development, give both commands the same `CLC_STATE_DIR` and use the same native build. Disable other installed CLC variants while verifying one variant so identical tool names do not select the wrong installation. Stopping the command stops rebuilding and closes its lease; an active host plugin can retain its own Core lease and the last built panel. If the generated file is removed, reopening uses the binary's embedded panel. Development and release resources use different URIs.

```sh
npm run plugin:build                 # native release ZIP and checksum
npm run plugin:build -- --debug      # debug artifact, embedded UI
npm run plugin:check -- <plugin-directory>
```

Build output is under `dist/plugin-package`. Packaging checks the version, executable, MCP config, tools, embedded UI, standalone Core and last-client shutdown, then extracts and checks the ZIP again. These checks do not establish acceptance of ChatGPT's real panel sandbox, automatic reload or Windows host behavior; exercise those in the target host separately.

## Export a standalone build

```sh
local-connector export <new-plugin-directory>
```

The exporter copies the executable and embedded UI into a self-contained plugin with a relative MCP command. It replaces recognized CLC exports and rejects unrelated directories. Exporting does not register a marketplace or install the plugin. The source template alone is not an installable artifact.
