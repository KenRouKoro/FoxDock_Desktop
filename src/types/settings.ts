/** 与后端 `SystemSettings` 对齐；可扩展更多字段 */
export type LanguagePreference = "system" | "zh" | "en";

export interface SystemSettings {
  languagePreference: LanguagePreference;
  debugEnabled: boolean;
  autoCheckUpdate: boolean;
}

export const DEFAULT_SYSTEM_SETTINGS: SystemSettings = {
  languagePreference: "system",
  debugEnabled: false,
  autoCheckUpdate: true,
};

export const SYSTEM_SETTINGS_INJECTION_KEY = Symbol("systemSettings");
