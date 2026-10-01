# ChatGPT desktop companion plugin

CLC's companion plugin packages a task workflow skill with a reference to an existing connected CLC App. It uses that App's current authentication, ingress tool policy and shared task namespace. It does not install another Agent runtime, create a new tunnel, or provide local direct transport.

The source template is `plugins/clc`. Its skill can use tools already available in the conversation. To bind a distributable local copy to a device's registered App, use the development exporter:

```sh
node tooling/export-plugin.mjs --app-id <existing-app-id-or-plugin-detail-url> --output <new-parent-directory>/clc
```

The exporter accepts the App ID or its ChatGPT plugin detail URL, copies the template and writes `.app.json` only to the output directory. Keep device App IDs and private connection settings outside the repository. No API key or Tunnel identity belongs in the plugin.

Expose that folder through a local personal marketplace, then install **Local Connector** from the ChatGPT desktop plugin directory. Local marketplace source paths resolve from the marketplace root; a personal entry `./plugins/clc` resolves to the user's `plugins/clc` directory. Preserve other entries. Restart the host if the local source has not appeared, and use a new conversation after installation or updates. Refer to [OpenAI's plugin packaging guide](https://developers.openai.com/plugins/build/plugins) for current marketplace registration and installation.

The registered App must already be accessible to the user. Plugin installation is not inbound connection verification. Web/mobile access and workspace plugin availability remain controlled by ChatGPT; a desktop-local marketplace does not install a plugin on those surfaces.

## Task workflow

The skill guides project discovery, Agent selection, task continuation, bounded waiting, and receipt recovery through the existing MCP tools. Task identity and ownership stay with the original Agent and execution host.

`agent_read` returns task state and recorded output. Large results remain available through the existing output readers. An operation receipt completing does not mean the Agent task has finished, and the Agent's final message does not independently verify the result.

The plugin provides no background subscription or wakeup mechanism. Following a task uses bounded reads and waits during the current conversation, or an available host scheduling feature when the user requests it.
