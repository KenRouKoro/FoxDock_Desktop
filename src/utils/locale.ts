import type { LanguagePreference } from "../types/settings";

export type AppLocale = "zh" | "en";

function systemLocales(): string[] {
  if (typeof navigator === "undefined") return [];
  const list = navigator.languages?.length
    ? [...navigator.languages]
    : [navigator.language];
  return list.filter(Boolean) as string[];
}

/** 将 BCP 47 语言标签映射为应用支持的 locale */
export function mapSystemLanguageToLocale(): AppLocale {
  for (const tag of systemLocales()) {
    const lower = tag.toLowerCase();
    if (lower.startsWith("zh")) {
      return "zh";
    }
  }
  return "en";
}

export function resolveLocaleFromPreference(
  preference: LanguagePreference,
): AppLocale {
  if (preference === "zh") return "zh";
  if (preference === "en") return "en";
  return mapSystemLanguageToLocale();
}
