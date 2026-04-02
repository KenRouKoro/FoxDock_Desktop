import { createApp, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import App from "./App.vue";
import i18n from "./i18n";
import "./styles/theme.css";
import {
  DEFAULT_SYSTEM_SETTINGS,
  SYSTEM_SETTINGS_INJECTION_KEY,
  type SystemSettings,
} from "./types/settings";
import { resolveLocaleFromPreference } from "./utils/locale";

window.addEventListener("contextmenu", (event) => {
  event.preventDefault();
});

async function bootstrap() {
  let settings: SystemSettings = { ...DEFAULT_SYSTEM_SETTINGS };
  try {
    const loaded = await invoke<SystemSettings>("load_system_settings");
    settings = { ...DEFAULT_SYSTEM_SETTINGS, ...loaded };
  } catch {
    /* 非 Tauri 环境或命令不可用时使用默认 */
  }

  const systemSettings = ref<SystemSettings>(settings);
  i18n.global.locale.value = resolveLocaleFromPreference(
    settings.languagePreference,
  );

  const app = createApp(App);
  app.use(i18n);
  app.provide(SYSTEM_SETTINGS_INJECTION_KEY, systemSettings);
  app.mount("#app");
}

void bootstrap();
