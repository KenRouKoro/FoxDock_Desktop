/** HID 控制台 `rssi_scan` 应答解析结果（与固件英文输出格式对齐） */

export type RssiChannelSample = {
  channel: number;
  dbm: number;
};

export type RssiScanParsed = {
  /** 原始完整文本 */
  raw: string;
  /** `RSSI scan ...` 至 `Current channel` 之前的参数摘要 */
  scanSummary: string;
  currentChannel: number | null;
  recommendedChannel: number | null;
  recommendedDbm: number | null;
  /** 按固件输出顺序（通常为 RSSI 从优到劣） */
  channels: RssiChannelSample[];
};

const LIST_SPLIT =
  /All channels sorted by RSSI\s*(?:\([^)]*\))?\s*:\s*/i;

/**
 * 解析 `rssi_scan` 多行/单行输出；无法识别时返回 `null`（调用方回退为纯文本展示）。
 */
export function parseRssiScanMessage(raw: string): RssiScanParsed | null {
  const s = raw.trim();
  if (!/RSSI scan/i.test(s) || !/All channels sorted by RSSI/i.test(s)) {
    return null;
  }

  const currentM = /Current channel \(effective\):\s*(\d+)/i.exec(s);
  const recM = /Recommended:\s*channel\s+(\d+)\s*\(\s*(-?\d+)\s*dBm\s*\)/i.exec(s);

  const listMatch = s.split(LIST_SPLIT);
  const listPart = listMatch.length >= 2 ? listMatch[listMatch.length - 1].trim() : "";

  const channels: RssiChannelSample[] = [];
  const pairRe = /(\d+)\s*\(\s*(-?\d+)\s*dBm\s*\)/gi;
  let m: RegExpExecArray | null;
  while ((m = pairRe.exec(listPart)) !== null) {
    channels.push({
      channel: Number.parseInt(m[1], 10),
      dbm: Number.parseInt(m[2], 10),
    });
  }

  if (channels.length === 0) {
    return null;
  }

  let scanSummary = s;
  const cutIdx = s.search(/Current channel\s*\(effective\):/i);
  if (cutIdx > 0) {
    scanSummary = s.slice(0, cutIdx).trim();
  }

  return {
    raw: s,
    scanSummary,
    currentChannel: currentM ? Number.parseInt(currentM[1], 10) : null,
    recommendedChannel: recM ? Number.parseInt(recM[1], 10) : null,
    recommendedDbm: recM ? Number.parseInt(recM[2], 10) : null,
    channels,
  };
}

/**
 * 解析 `list` 应答中的 `Stored devices:` MAC 列表（12 位十六进制，空格分隔）。
 */
export function parseStoredDevicesList(raw: string): string[] | null {
  const s = raw.trim();
  const block = /Stored devices:\s*((?:[0-9A-Fa-f]{12}(?:\s+|$))+)/i.exec(s);
  if (!block?.[1]) {
    return null;
  }
  const macs = block[1]
    .trim()
    .split(/\s+/)
    .filter((t) => /^[0-9A-Fa-f]{12}$/.test(t));
  return macs.length > 0 ? macs : null;
}

/** 按样本集将 dBm 映射为 0（最优，数值最小）…1（最差） */
export function rssiDbmHeatRatio(dbm: number, minDbm: number, maxDbm: number): number {
  if (maxDbm <= minDbm) return 0;
  return (dbm - minDbm) / (maxDbm - minDbm);
}
