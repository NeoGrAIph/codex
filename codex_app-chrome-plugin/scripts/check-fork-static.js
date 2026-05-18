#!/usr/bin/env node
/* global console, process */

const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const EXPECTED_EXTENSION_NAME = "Codex App Chrome";
const EXPECTED_HOST_NAME = "com.neograiph.codexappchrome";
const EXTENSION_ROOT_ENV = "CODEX_CHROME_EXTENSION_ROOT";
const EXTENSION_HOST_PATH_ENV = "CODEX_CHROME_EXTENSION_HOST_PATH";
const OFFICIAL_IDENTIFIERS = [
  "hehggadaopoacecdllhhajmbjkdcmajg",
  "com.openai.codexextension",
  "chromewebstore.google.com/detail/codex",
  "developers.openai.com/codex/app/chrome-extension",
  "cdn.openai.com",
];
const REQUIRED_BACKGROUND_METHODS = [
  "ping",
  "executeCdp",
  "attach",
  "detach",
  "getTabs",
  "getUserTabs",
  "getUserHistory",
  "claimUserTab",
  "createTab",
  "finalizeTabs",
  "nameSession",
  "executeUnhandledCommand",
  "moveMouse",
  "turnEnded",
  "getInfo",
];

function usage() {
  console.error("Usage: scripts/check-fork-static.js [--json]");
  console.error("");
  console.error(
    `Optional extension root override: ${EXTENSION_ROOT_ENV}=/path/to/unpacked-extension`,
  );
}

function scriptPath(filename) {
  return path.resolve(__dirname, filename);
}

function pluginRoot() {
  return path.resolve(__dirname, "..");
}

function defaultExtensionRoot() {
  return path.resolve(pluginRoot(), "..", "codex-chrome-extension-unpacked");
}

function extensionRoot() {
  const override = process.env[EXTENSION_ROOT_ENV];
  return override ? path.resolve(override) : defaultExtensionRoot();
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function readText(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function deriveExtensionId(manifestKey) {
  const der = Buffer.from(manifestKey, "base64");
  const hash = crypto.createHash("sha256").update(der).digest();
  return [...hash.subarray(0, 16)]
    .map((byte) => {
      return (
        String.fromCharCode(97 + (byte >> 4)) +
        String.fromCharCode(97 + (byte & 15))
      );
    })
    .join("");
}

function requireIncludes(problems, sourceName, source, expected) {
  if (!source.includes(expected)) {
    problems.push(`${sourceName} does not include ${expected}`);
  }
}

function rejectIncludes(problems, sourceName, source, forbidden) {
  if (source.includes(forbidden)) {
    problems.push(`${sourceName} still includes ${forbidden}`);
  }
}

function hostBinaryPath() {
  if (process.env[EXTENSION_HOST_PATH_ENV]) {
    return path.resolve(process.env[EXTENSION_HOST_PATH_ENV]);
  }

  const platformDirectories = {
    darwin: "macos",
    linux: "linux",
    win32: "windows",
  };
  const platformDirectory = platformDirectories[process.platform];
  if (!platformDirectory) return null;

  const executable =
    platformDirectory === "windows" ? "extension-host.exe" : "extension-host";
  return path.resolve(
    pluginRoot(),
    "extension-host",
    platformDirectory,
    process.arch,
    executable,
  );
}

function checkStatic() {
  const problems = [];
  const runtimeBlockers = [];
  const root = extensionRoot();
  const manifestPath = path.resolve(root, "manifest.json");
  const backgroundPath = path.resolve(root, "background.js");
  const popupBundlePath = path.resolve(root, "chunks", "popup-47nmQnIc.js");
  const contentScriptPath = path.resolve(root, "content-scripts", "codex.js");
  const configPath = scriptPath("extension-id.json");
  const installManifestPath = scriptPath("installManifest.mjs");
  const browserClientPath = scriptPath("browser-client.mjs");

  const manifest = readJson(manifestPath);
  const config = readJson(configPath);
  const background = readText(backgroundPath);
  const popupBundle = readText(popupBundlePath);
  const installManifest = readText(installManifestPath);
  const browserClient = readText(browserClientPath);

  if (manifest.name !== EXPECTED_EXTENSION_NAME) {
    problems.push(`manifest name is ${JSON.stringify(manifest.name)}`);
  }
  if (manifest.action?.default_title !== EXPECTED_EXTENSION_NAME) {
    problems.push(
      `manifest action.default_title is ${JSON.stringify(
        manifest.action?.default_title,
      )}`,
    );
  }
  if (manifest.update_url != null) {
    problems.push("manifest still declares update_url");
  }
  if (
    manifest.content_security_policy?.extension_pages?.includes(
      "cdn.openai.com",
    )
  ) {
    problems.push("manifest CSP still allows cdn.openai.com");
  }

  const derivedExtensionId = deriveExtensionId(manifest.key);
  if (derivedExtensionId !== config.extensionId) {
    problems.push(
      `derived extension id ${derivedExtensionId} does not match ${config.extensionId}`,
    );
  }
  if (config.extensionHostName !== EXPECTED_HOST_NAME) {
    problems.push(
      `extensionHostName is ${JSON.stringify(config.extensionHostName)}`,
    );
  }

  requireIncludes(problems, "background.js", background, EXPECTED_HOST_NAME);
  requireIncludes(problems, "popup bundle", popupBundle, EXPECTED_EXTENSION_NAME);
  requireIncludes(
    problems,
    "installManifest.mjs",
    installManifest,
    "extension-id.json",
  );
  requireIncludes(problems, "browser-client.mjs", browserClient, "sendSessionRequest");
  if (!fs.existsSync(contentScriptPath)) {
    problems.push(`missing content script: ${contentScriptPath}`);
  }

  for (const method of REQUIRED_BACKGROUND_METHODS) {
    requireIncludes(problems, "background.js", background, method);
  }
  for (const forbidden of OFFICIAL_IDENTIFIERS) {
    rejectIncludes(problems, "background.js", background, forbidden);
    rejectIncludes(problems, "popup bundle", popupBundle, forbidden);
    rejectIncludes(problems, "installManifest.mjs", installManifest, forbidden);
  }

  const nativeHostPath = hostBinaryPath();
  if (process.platform === "linux" && !fs.existsSync(nativeHostPath)) {
    runtimeBlockers.push(
      `missing Linux native host binary: ${nativeHostPath}; set ${EXTENSION_HOST_PATH_ENV} during install or add this binary`,
    );
  }

  return {
    ok: problems.length === 0,
    problems,
    runtimeReady: runtimeBlockers.length === 0,
    runtimeBlockers,
    extensionRoot: root,
    extensionName: manifest.name,
    extensionId: config.extensionId,
    derivedExtensionId,
    extensionHostName: config.extensionHostName,
    nativeHostPath,
  };
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes("-h") || args.includes("--help")) {
    usage();
    return;
  }

  const json = args.includes("--json");
  const positionalArgs = args.filter((arg) => arg !== "--json");
  if (positionalArgs.length > 0) {
    usage();
    process.exit(2);
  }

  const result = checkStatic();
  if (json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(`Extension root: ${result.extensionRoot}`);
    console.log(`Extension name: ${result.extensionName}`);
    console.log(`Extension ID: ${result.extensionId}`);
    console.log(`Native host name: ${result.extensionHostName}`);
    console.log(`Static contract: ${result.ok ? "ok" : "failed"}`);
    if (result.problems.length > 0) {
      console.log(`Problems: ${result.problems.join("; ")}`);
    }
    console.log(`Runtime ready: ${result.runtimeReady ? "yes" : "no"}`);
    if (result.runtimeBlockers.length > 0) {
      console.log(`Runtime blockers: ${result.runtimeBlockers.join("; ")}`);
    }
  }

  process.exit(result.ok ? 0 : 1);
}

main();
