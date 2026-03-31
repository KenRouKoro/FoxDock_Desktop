/**
 * 以根目录 package.json 的 version 为唯一来源，同步到 Tauri 与 Cargo 配置。
 * 用法: node scripts/sync-version.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

function readPackageVersion() {
  const pkgPath = join(root, "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  const v = pkg.version;
  if (!v || typeof v !== "string") {
    throw new Error("package.json: missing or invalid \"version\"");
  }
  return v.trim();
}

function syncTauriConf(version) {
  const path = join(root, "src-tauri", "tauri.conf.json");
  const data = JSON.parse(readFileSync(path, "utf8"));
  data.version = version;
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

const version = readPackageVersion();
syncTauriConf(version);
syncCargoToml(version);
console.log(`sync-version: ${version} -> tauri.conf.json, Cargo.toml`);
