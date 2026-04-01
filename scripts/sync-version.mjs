/**
 * 派生真实应用版本（见 scripts/version-utils.mjs），并同步到 Tauri 与 Cargo 配置。
 * 用法: node scripts/sync-version.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deriveAppVersion, deriveMsiProductVersion } from "./version-utils.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

function syncTauriConf(version, msiProductVersion) {
  const path = join(root, "src-tauri", "tauri.conf.json");
  const data = JSON.parse(readFileSync(path, "utf8"));
  data.version = version;
  data.bundle = data.bundle ?? {};
  data.bundle.windows = data.bundle.windows ?? {};
  data.bundle.windows.wix = { ...(data.bundle.windows.wix ?? {}), version: msiProductVersion };
  writeFileSync(path, JSON.stringify(data, null, 2) + "\n", "utf8");
}

function syncCargoToml(version) {
  const path = join(root, "src-tauri", "Cargo.toml");
  const lines = readFileSync(path, "utf8").split(/\r?\n/);
  let inPackage = false;
  let replaced = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (trimmed === "[package]") {
      inPackage = true;
      continue;
    }
    if (trimmed.startsWith("[") && trimmed !== "[package]") {
      inPackage = false;
    }
    if (inPackage && /^version\s*=/.test(line)) {
      lines[i] = `version = "${version}"`;
      replaced = true;
      break;
    }
  }
  if (!replaced) {
    throw new Error("Cargo.toml: could not find [package] version = line");
  }
  writeFileSync(path, lines.join("\n"), "utf8");
}

const version = deriveAppVersion(root);
const msiProductVersion = deriveMsiProductVersion(root);
syncTauriConf(version, msiProductVersion);
syncCargoToml(version);
console.log(`sync-version: ${version} (MSI ${msiProductVersion}) -> tauri.conf.json, Cargo.toml`);
