import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";
import { applyTheme } from "./app/theme";
import { loadPreferences } from "./app/prefs";

// 首帧前应用已保存主题，避免闪白/闪黑；"跟随系统" 由 theme 模块解析。
applyTheme(loadPreferences().theme);

createApp(App).mount("#app");
