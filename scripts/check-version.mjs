/**
 * 校验 package.json、tauri.conf.json、Cargo.toml 三处版本号是否一致。
 * 一致则退出 0，否则退出 1。
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");

function pkgVersion() {
  const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  return String(pkg.version ?? "").trim();
}

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

const a = pkgVersion();
const b = tauriVersion();
const c = cargoVersion();

if (a === b && b === c) {
  console.log(`check-version: OK (${a})`);
  process.exit(0);
}

console.error("check-version: mismatch");
console.error(`  package.json:     ${a}`);
console.error(`  tauri.conf.json:  ${b}`);
console.error(`  Cargo.toml:       ${c}`);
console.error('Run "pnpm run sync:version" to align from package.json.');
process.exit(1);
