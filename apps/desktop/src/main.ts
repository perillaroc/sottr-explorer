import { createApp } from "vue";
import App from "./App.vue";
import DetailWindow from "./components/DetailWindow.vue";
import "./styles.css";
import { applyTheme } from "./app/theme";
import { loadPreferences } from "./app/prefs";
import { setLanguage } from "./app/i18n";
import { isDetailPopupWindow } from "./app/detail-window";

// 首帧前应用已保存主题，避免闪白/闪黑；"跟随系统" 由 theme 模块解析。
const preferences = loadPreferences();
applyTheme(preferences.theme);
setLanguage(preferences.language);

// 详情弹窗入口（阶段三）：Tauri 以窗口标签 `detail`、浏览器以 `#popup=detail` 识别；
// 弹窗只挂载 DetailWindow（读取 Rust 内存/sessionStorage 的只读 payload），不创建 store。
if (isDetailPopupWindow()) {
  createApp(DetailWindow, { mode: "window" }).mount("#app");
} else {
  createApp(App).mount("#app");
}
