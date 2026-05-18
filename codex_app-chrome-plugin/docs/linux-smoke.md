# Codex App Chrome Linux smoke runbook

This runbook validates the forked Chrome extension and Codex App plugin as a side-by-side local setup on Linux x64.

## Fixed fork identity

- Extension name: `Codex App Chrome`
- Extension ID: `mlpdilgdbnefonpojdicmagfmogikfmj`
- Native host name: `com.neograiph.codexappchrome`
- Plugin name: `codex-app-chrome`

Do not change JSON-RPC method names, content-script message names, or the browser-client API while running this smoke.

## Prerequisites

- Google Chrome installed.
- A Linux x64 Browser Use `extension-host` binary.
- The unpacked extension directory at `../codex-chrome-extension-unpacked` relative to this plugin directory, or `CODEX_CHROME_EXTENSION_ROOT` set to the unpacked extension path.

This fork currently does not bundle `extension-host/linux/x64/extension-host`. Use one of these options:

- Add the real binary at `extension-host/linux/x64/extension-host`.
- Keep the binary elsewhere and set `CODEX_CHROME_EXTENSION_HOST_PATH=/absolute/path/to/extension-host` during native-host installation.

Do not use a stub host for runtime smoke testing; a stub can only validate manifest shape.

## Static checks

Run from the plugin root:

```bash
scripts/check-fork-static.js --json
node scripts/installManifest.mjs --dry-run --json
jq . ../codex-chrome-extension-unpacked/manifest.json
jq . scripts/extension-id.json
node --check scripts/installManifest.mjs
node --check scripts/check-fork-static.js
```

`check-fork-static.js` may report `runtimeReady: false` until the Linux native host binary is present. Static readiness is the `ok` field. If the host binary is external, run the checker with `CODEX_CHROME_EXTENSION_HOST_PATH=/absolute/path/to/extension-host`.

## Load the unpacked extension

Use a dedicated Chrome profile for the smoke:

```bash
google-chrome \
  --user-data-dir=/tmp/codex-app-chrome-profile \
  --load-extension="$PWD/../codex-chrome-extension-unpacked" \
  about:blank
```

Open `chrome://extensions`, enable Developer Mode if needed, and confirm the loaded extension ID is `mlpdilgdbnefonpojdicmagfmogikfmj`.

For plugin scripts that inspect this test profile, use:

```bash
export CODEX_CHROME_USER_DATA_DIR=/tmp/codex-app-chrome-profile
```

## Install native messaging manifest

If the Linux host binary is bundled in the plugin:

```bash
node scripts/installManifest.mjs
```

If the host binary is external:

```bash
CODEX_CHROME_EXTENSION_HOST_PATH=/absolute/path/to/extension-host \
  node scripts/installManifest.mjs
```

Then verify:

```bash
scripts/check-native-host-manifest.js --json
scripts/check-extension-installed.js --json
```

Expected native-host manifest path on Linux:

```text
~/.config/google-chrome/NativeMessagingHosts/com.neograiph.codexappchrome.json
```

## Runtime smoke

After Chrome is running with the unpacked extension and the native host manifest is correct:

1. Open the extension popup and confirm it reports connected.
2. Bootstrap `scripts/browser-client.mjs` from the Codex App plugin runtime.
3. Run `await agent.browsers.list()` and confirm an `extension` backend appears.
4. Run `browser.tabs.list()`, `browser.tabs.new()`, `browser.nameSession("Smoke")`, a cursor move, and `browser.tabs.finalize({ keep: [] })`.
5. Confirm created tabs appear in a `Codex App` Chrome tab group and finalization cleans up agent-created tabs.

## Failure interpretation

- `check-fork-static.js` fails: fix fork identity or accidental official-extension references first.
- `check-extension-installed.js` fails: Chrome profile does not have the unpacked extension registered/enabled.
- `check-native-host-manifest.js` fails: native host manifest name, allowed origin, registry/path, or host binary path is wrong.
- Popup remains disconnected after install checks pass: restart Chrome with the dedicated profile and retry the browser-client bootstrap once.
