import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { arch, homedir, platform } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const EXTENSION_HOST_PATH_ENV = "CODEX_CHROME_EXTENSION_HOST_PATH";
const PLATFORM_DIRECTORIES = {
  darwin: "macos",
  linux: "linux",
  win32: "windows",
};
const execFileAsync = promisify(execFile);
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const scriptPath = fileURLToPath(import.meta.url);

async function loadExtensionConfig() {
  const configPath = path.resolve(scriptDir, "extension-id.json");
  const config = JSON.parse(await readFile(configPath, "utf8"));
  if (
    !config ||
    typeof config.extensionId !== "string" ||
    typeof config.extensionHostName !== "string"
  ) {
    throw new Error(`Invalid Chrome extension config: ${configPath}.`);
  }
  return config;
}

function resolveExtensionHostPath(pluginRoot) {
  const overridePath = process.env[EXTENSION_HOST_PATH_ENV];
  if (overridePath) return path.resolve(overridePath);

  const currentPlatform = platform();
  const currentArch = arch();
  const platformDirectory = PLATFORM_DIRECTORIES[currentPlatform];
  if (!platformDirectory || (currentArch !== "arm64" && currentArch !== "x64")) {
    throw new Error(
      `Invalid platform or architecture: ${currentPlatform} ${currentArch}`,
    );
  }

  const executable =
    platformDirectory === "windows" ? "extension-host.exe" : "extension-host";
  return path.resolve(
    pluginRoot,
    `extension-host/${platformDirectory}/${currentArch}/${executable}`,
  );
}

async function assertExtensionHostPresent(pluginRoot) {
  const hostPath = resolveExtensionHostPath(pluginRoot);
  if (!existsSync(hostPath)) {
    throw new Error(
      `Missing Browser Use extension host binary at ${hostPath}. Set ${EXTENSION_HOST_PATH_ENV}=/path/to/extension-host to use an external host binary.`,
    );
  }
}

async function buildInstallPlan() {
  const config = await loadExtensionConfig();
  const pluginRoot = resolvePluginCacheLatest(path.resolve(scriptDir, ".."));
  const hostPath = resolveExtensionHostPath(pluginRoot);
  const manifestPaths = nativeHostManifestPaths(config);

  return {
    platform: platform(),
    arch: arch(),
    pluginRoot,
    extensionHostPath: hostPath,
    extensionHostPathExists: existsSync(hostPath),
    manifestPaths,
    extensionId: config.extensionId,
    extensionHostName: config.extensionHostName,
    allowedOrigin: `chrome-extension://${config.extensionId}/`,
  };
}

function buildNativeHostManifest(pluginRoot, config) {
  return {
    name: config.extensionHostName,
    description: "Codex App Chrome native messaging host",
    type: "stdio",
    path: resolveExtensionHostPath(pluginRoot),
    allowed_origins: [`chrome-extension://${config.extensionId}/`],
  };
}

function nativeHostManifestPaths(config) {
  const filename = `${config.extensionHostName}.json`;
  const relativeDirectories = {
    darwin: ["Library/Application Support/Google/Chrome/NativeMessagingHosts"],
    linux: [".config/google-chrome/NativeMessagingHosts"],
    win32: ["AppData/Local/CodexApp/extension"],
  }[platform()];
  if (!relativeDirectories) {
    throw new Error(`Unsupported platform: ${platform()}`);
  }

  return relativeDirectories.map((directory) =>
    path.resolve(homedir(), directory, filename),
  );
}

async function writeNativeHostManifest(pluginRoot, config) {
  const manifest = buildNativeHostManifest(pluginRoot, config);
  const manifestJson = `${JSON.stringify(manifest, null, 2)}\n`;
  await Promise.all(
    nativeHostManifestPaths(config).map(async (manifestPath) => {
      await mkdir(path.dirname(manifestPath), { recursive: true });
      await writeFile(manifestPath, manifestJson);
    }),
  );
}

function resolvePluginCacheLatest(pluginRoot) {
  const parts = path.resolve(pluginRoot).split(path.sep);
  const cacheIndex = parts.lastIndexOf("cache");
  if (
    cacheIndex < 1 ||
    parts[cacheIndex - 1] !== "plugins" ||
    parts.length <= cacheIndex + 3
  ) {
    return pluginRoot;
  }
  return path.resolve(pluginRoot, "..", "latest");
}

async function writeWindowsRegistry(config) {
  if (platform() !== "win32") return;

  const registryKey = `HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts\\${config.extensionHostName}`;
  const manifestPath = nativeHostManifestPaths(config)[0];
  if (!manifestPath) throw new Error("Invalid Windows path returned");

  await execFileAsync("reg", [
    "add",
    registryKey,
    "/ve",
    "/t",
    "REG_SZ",
    "/d",
    manifestPath,
    "/f",
  ]);
}

async function install() {
  const config = await loadExtensionConfig();
  const pluginRoot = resolvePluginCacheLatest(path.resolve(scriptDir, ".."));
  await assertExtensionHostPresent(pluginRoot);
  await writeNativeHostManifest(pluginRoot, config);
  await writeWindowsRegistry(config);
}

function usage() {
  console.error("Usage: scripts/installManifest.mjs [--dry-run] [--json]");
  console.error("");
  console.error(
    `Set ${EXTENSION_HOST_PATH_ENV}=/path/to/extension-host to use an external native host binary.`,
  );
}

async function main() {
  const args = process.argv.slice(2);
  if (args.includes("-h") || args.includes("--help")) {
    usage();
    return;
  }

  const dryRun = args.includes("--dry-run");
  const json = args.includes("--json");
  const positionalArgs = args.filter(
    (arg) => arg !== "--dry-run" && arg !== "--json",
  );
  if (positionalArgs.length > 0) {
    usage();
    process.exitCode = 2;
    return;
  }

  const plan = await buildInstallPlan();
  if (!dryRun) await install();

  const result = { ...plan, installed: !dryRun };
  if (json) {
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  console.log(`Platform: ${result.platform}/${result.arch}`);
  console.log(`Extension ID: ${result.extensionId}`);
  console.log(`Native host name: ${result.extensionHostName}`);
  console.log(`Host binary: ${result.extensionHostPath}`);
  console.log(`Host binary exists: ${result.extensionHostPathExists ? "yes" : "no"}`);
  console.log(`Manifest paths: ${result.manifestPaths.join(", ")}`);
  console.log(`Installed: ${result.installed ? "yes" : "no"}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}

export { buildInstallPlan, install };
