// 主题（SX-015）：跟随系统 / 浅色 / 深色；由 <html data-theme> 驱动令牌切换。

export type ThemePreference = "system" | "light" | "dark";

export function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(prefers-color-scheme: dark)").matches
    : false;
}

export function resolveTheme(preference: ThemePreference): "light" | "dark" {
  if (preference === "system") {
    return systemPrefersDark() ? "dark" : "light";
  }
  return preference;
}

export function applyTheme(preference: ThemePreference): void {
  if (typeof document === "undefined") {
    return;
  }
  const root = document.documentElement;
  root.setAttribute("data-theme", resolveTheme(preference));
  root.setAttribute("data-theme-preference", preference);
}

/** 订阅系统主题变化；返回取消订阅函数。 */
export function onSystemThemeChange(listener: () => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return () => {};
  }
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", listener);
  return () => media.removeEventListener("change", listener);
}
