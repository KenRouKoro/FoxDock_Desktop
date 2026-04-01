/**
 * 校验 tauri.conf.json、Cargo.toml 中的版本号等于当前派生的完整应用版本（与 sync-version 同源）。
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deriveAppVersion, readPackageBaseVersion } from "./version-utils.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

function tauriVersion() {
  const data = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
  return String(data.version ?? "").trim();
}

function cargoVersion() {
  const text = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
  const lines = text.split(/\r?\n/);
  let inPackage = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === "[package]") {
      inPackage = true;
      continue;
    }
    if (trimmed.startsWith("[") && trimmed !== "[package]") {
      inPackage = false;
    }
    if (inPackage) {
      const m = line.match(/^\s*version\s*=\s*"([^"]*)"/);
      if (m) return m[1].trim();
    }
  }
  throw new Error("Cargo.toml: [package] version not found");
}

const base = readPackageBaseVersion(root);
let expected;
try {
  expected = deriveAppVersion(root);
} catch (e) {
  console.error("check-version: failed to derive expected app version");
  console.error(e instanceof Error ? e.message : e);
  process.exit(1);
}

const b = tauriVersion();
const c = cargoVersion();

if (b === expected && c === expected) {
  console.log(`check-version: OK (${expected}, package.json base ${base} for npm only)`);
  process.exit(0);
}

console.error("check-version: mismatch");
console.error(`  package.json base:  ${base}`);
console.error(`  expected (derived): ${expected}`);
console.error(`  tauri.conf.json:    ${b}`);
console.error(`  Cargo.toml:         ${c}`);
console.error('Run "pnpm run sync:version" to align with current env + git HEAD.');
process.exit(1);
