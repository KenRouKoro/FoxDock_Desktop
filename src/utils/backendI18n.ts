import type { ComposerTranslation } from "vue-i18n";

const BACKEND_I18N_PREFIX = "i18n:";

/** Resolve `i18n:key` or `i18n:key|{"param":"..."}` strings from the Tauri backend. */
export function resolveBackendI18nMessage(
  t: ComposerTranslation,
  raw: string,
): string | null {
  if (!raw.startsWith(BACKEND_I18N_PREFIX)) return null;
  const payload = raw.slice(BACKEND_I18N_PREFIX.length);
  const separatorIndex = payload.indexOf("|");
  const key = separatorIndex === -1 ? payload : payload.slice(0, separatorIndex);
  if (!key) return null;
  if (separatorIndex === -1) {
    return String(t(key));
  }
  const paramsText = payload.slice(separatorIndex + 1);
  try {
    const parsed = JSON.parse(paramsText) as Record<string, unknown>;
    const resolvedParams = Object.fromEntries(
      Object.entries(parsed).map(([paramKey, paramValue]) => {
        if (typeof paramValue === "string") {
          return [paramKey, resolveBackendI18nMessage(t, paramValue) ?? paramValue];
        }
        return [paramKey, paramValue];
      }),
    );
    return String(t(key, resolvedParams));
  } catch {
    return String(t(key));
  }
}

export function resolveMessage(t: ComposerTranslation, raw: string): string {
  return resolveBackendI18nMessage(t, raw) ?? raw;
}
