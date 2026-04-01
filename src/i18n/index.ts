import { createI18n } from 'vue-i18n';
import zh from './locales/zh';
import en from './locales/en';

const i18n = createI18n({
  legacy: false, // 使用 Composition API
  /** 由 main.ts 在挂载前按持久化/系统语言覆盖 */
  locale: 'en',
  fallbackLocale: 'en',
  messages: {
    zh,
    en
  }
});

export default i18n;
