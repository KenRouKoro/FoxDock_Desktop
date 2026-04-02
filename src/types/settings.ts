/** 与后端 `SystemSettings` 对齐；可扩展更多字段 */
export type LanguagePreference = "system" | "zh" | "en";

export interface SystemSettings {
  languagePreference: LanguagePreference;
  debugEnabled: boolean;
  autoCheckUpdate: boolean;
  autoDockOnStartup: boolean;
  dockAlwaysOnTop: boolean;
  followSlimeVrWindow: boolean;
  snapStyleApproximation: boolean;
}

export const DEFAULT_SYSTEM_SETTINGS: SystemSettings = {
  languagePreference: "system",
  debugEnabled: false,
  autoCheckUpdate: true,
  autoDockOnStartup: true,
  dockAlwaysOnTop: false,
  followSlimeVrWindow: false,
  snapStyleApproximation: false,
};

export const SYSTEM_SETTINGS_INJECTION_KEY = Symbol("systemSettings");
