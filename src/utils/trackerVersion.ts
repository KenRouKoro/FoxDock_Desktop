/** 与固件 `format_usb_build_info_v1` 一致：`{major}.{minor}.{patch}+{tweak} {hash}`，可选末尾 ` dirty`。哈希放第二行展示。 */
export function parseTrackerVersion(version: string): {
  mainLine: string;
  hashLine: string | null;
} {
  const dirtySuffix = version.endsWith(" dirty") ? " dirty" : "";
  const core = dirtySuffix ? version.slice(0, -" dirty".length) : version;
  const lastSpace = core.lastIndexOf(" ");
  if (lastSpace === -1) {
    return { mainLine: version, hashLine: null };
  }
  const tail = core.slice(lastSpace + 1);
  if (/^[0-9a-fA-F]{6,}$/.test(tail)) {
    return {
      mainLine: core.slice(0, lastSpace),
      hashLine: tail + dirtySuffix,
    };
  }
  return { mainLine: version, hashLine: null };
}
