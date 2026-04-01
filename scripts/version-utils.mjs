/**
 * 共享：派生真实应用版本（SemVer，供 Cargo / Tauri 使用）。
 *
 * - 默认（本地与普通 CI）：`0.0.0-dev.<YYYYMMDD>.<HHmmss>.<shortHash>`（日期时间为 HEAD 提交时间的 UTC；无 package.json 基础前缀，含 dev 标识）
 * - GitHub Release 构建：设置环境变量 `FOXDOCK_RELEASE_TAG` 为 Release tag 时，
 *   `<tag 解析出的 semver>-<YYYYMMDD>.<HHmmss>.<shortHash>`（日期时间同上；无 dev 后缀；tag 原样参与解析，见 deriveReleaseAppVersion）
 */
import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

/** @type {string} 用于从 Release 工作流注入的 tag（仅 release 事件设置） */
export const RELEASE_TAG_ENV = "FOXDOCK_RELEASE_TAG";

/**
 * @param {string} root 仓库根目录
 * @returns {string} package.json 中的 version（npm 元数据；dev 派生不再使用其作为前缀）
 */
export function readPackageBaseVersion(root) {
  const pkgPath = join(root, "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  const v = pkg.version;
  if (!v || typeof v !== "string") {
    throw new Error('package.json: missing or invalid "version"');
  }
  return v.trim();
}

/**
 * 使用当前 HEAD **提交时间**（UTC）拆成 YYYYMMDD 与 HHmmss。
 * 与墙钟时间相比，同一提交上多次运行 sync/check 结果一致，避免 prebuild 内连续两次 sync 与随后 check 因跨秒不一致。
 *
 * @param {string} root 仓库根目录
 * @returns {{ ymd: string, hm: string }}
 */
export function getGitCommitDateTimeYYYYMMDD_HHmmssUTC(root) {
  const out = execSync("git show -s --format=%ct HEAD", {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const sec = parseInt(out.trim(), 10);
  if (Number.isNaN(sec)) {
    throw new Error("git show: could not parse commit timestamp");
  }
  const d = new Date(sec * 1000);
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, "0");
  const day = String(d.getUTCDate()).padStart(2, "0");
  const H = String(d.getUTCHours()).padStart(2, "0");
  const M = String(d.getUTCMinutes()).padStart(2, "0");
  const s = String(d.getUTCSeconds()).padStart(2, "0");
  return { ymd: `${y}${m}${day}`, hm: `${H}${M}${s}` };
}

/**
 * 当前 HEAD 的短提交哈希（需 git 可用且在 git 仓库内）。
 * @param {string} root 仓库根目录
 * @returns {string}
 */
export function getGitShortHash(root) {
  try {
    const out = execSync("git rev-parse --short HEAD", {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    const hash = out.trim();
    if (!hash) {
      throw new Error("git rev-parse returned empty hash");
    }
    return hash;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    throw new Error(
      `Failed to read git commit hash: ${msg}. Ensure git is installed and this directory is a git checkout.`
    );
  }
}

/**
 * 将 GitHub Release tag 解析为 semver 核心与可选的 tag 侧 prerelease，再拼接构建时间戳与 hash。
 * tag 保留原样参与解析：仅去掉前导 `v` 以符合 Cargo semver；其余片段写入 semver prerelease。
 *
 * @param {string} tag 例如 `v1.2.3` 或 `1.2.3-rc.1`
 * @param {string} ymd YYYYMMDD
 * @param {string} hm HHmmss
 * @param {string} hash git short hash
 * @returns {string}
 */
export function deriveReleaseAppVersion(tag, ymd, hm, hash) {
  const parsed = parseGitHubReleaseTag(tag);
  const stamp = `${ymd}.${hm}.${hash}`;
  if (parsed.prereleaseFromTag) {
    return `${parsed.core}-${parsed.prereleaseFromTag}.${stamp}`;
  }
  return `${parsed.core}-${stamp}`;
}

/**
 * @param {string} tag
 * @returns {{ core: string, prereleaseFromTag: string | null }}
 */
function parseGitHubReleaseTag(tag) {
  const t = tag.trim();
  if (!t) {
    throw new Error(`${RELEASE_TAG_ENV}: empty tag`);
  }
  const withoutV = t.startsWith("v") ? t.slice(1) : t;
  const m = withoutV.match(/^(\d+\.\d+\.\d+)(?:-([\w.+-]+))?$/);
  if (m) {
    return { core: m[1], prereleaseFromTag: m[2] ?? null };
  }
  const safe = withoutV
    .replace(/[^0-9A-Za-z.-]/g, ".")
    .replace(/\.+/g, ".")
    .replace(/^\.+|\.+$/g, "");
  const ident = safe || "build";
  return { core: "0.0.0", prereleaseFromTag: ident };
}

/**
 * 派生完整应用版本。
 * @param {string} root 仓库根目录
 * @returns {string}
 */
export function deriveAppVersion(root) {
  const hash = getGitShortHash(root);
  const { ymd, hm } = getGitCommitDateTimeYYYYMMDD_HHmmssUTC(root);
  const releaseTag = process.env[RELEASE_TAG_ENV]?.trim();
  if (releaseTag) {
    return deriveReleaseAppVersion(releaseTag, ymd, hm, hash);
  }
  return `0.0.0-dev.${ymd}.${hm}.${hash}`;
}
