import { createApp } from "vue";

import App from "./App.vue";
import { applyTheme, setLocale } from "./i18n";
import "./styles.css";

// 首帧先用默认语言/主题渲染，真实值在拿到后端快照后立即覆盖
setLocale("zh");
applyTheme("light");

createApp(App).mount("#app");
