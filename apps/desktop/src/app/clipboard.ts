// 剪贴板（SX-016）：复制已核验链接与可用线索。仅在用户点击动作时调用；
// 浏览器/WebView 均可能没有异步剪贴板权限，失败时返回 false 交由界面提示手动复制。
export async function copyText(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    return false;
  }
  return false;
}
