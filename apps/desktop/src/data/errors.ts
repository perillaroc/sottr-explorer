// 命令错误的归一化与恢复提示（SX-016）：Tauri `invoke` 拒绝时会给出命令序列化
// 的错误负载（`{ code, message }`，见 SX-014 `CommandError`）；网络层/未知异常
// 则给出字符串或 Error。界面只依赖 `{ code, message }`，因此统一在这里收敛。
//
// 说明：后端 `message` 已是面向用户的中文说明（不泄露私人路径之外的内部信息），
// 这里只按稳定错误码追加“下一步可做什么”的恢复提示，不替换后端原因。
import type { CommandErrorDto } from "./source";

/** 未知异常兜底错误码（不冒充后端稳定错误码）。 */
export const CODE_UNEXPECTED = "UNEXPECTED";

export function toCommandError(error: unknown): CommandErrorDto {
  if (typeof error === "object" && error !== null) {
    const candidate = error as { code?: unknown; message?: unknown };
    if (typeof candidate.code === "string" && candidate.code.length > 0) {
      return {
        code: candidate.code,
        message: typeof candidate.message === "string" ? candidate.message : candidate.code,
      };
    }
    if (error instanceof Error) {
      return { code: CODE_UNEXPECTED, message: error.message };
    }
  }
  if (typeof error === "string" && error.length > 0) {
    return { code: CODE_UNEXPECTED, message: error };
  }
  return { code: CODE_UNEXPECTED, message: "命令失败，未返回可读原因" };
}

/** 稳定错误码 → 恢复建议（没有对应建议返回 null）。 */
export function recoveryHint(error: CommandErrorDto): string | null {
  const hints: Record<string, string> = {
    OUTPUT_EXISTS: "换一个新目录名后重试（导出不会覆盖既有目录或文件）",
    OUTPUT_PARENT_NOT_DIRECTORY: "重新选择目标位置（父级必须是目录）",
    OUTPUT_INVALID_TARGET: "检查目录名是否包含非法字符或以空格/句点结尾",
    OUTPUT_LINK_REPLACED: "目标路径被替换为链接/重解析点，请选择其他位置",
    OUTPUT_WRITE_FAILED: "检查磁盘空间与目标权限后重试",
    OUTPUT_COMMIT_FAILED: "稍后重试；若持续失败请更换目标磁盘",
    INPUT_OUTPUT_OVERLAP: "把导出目标移出存档目录（含其子目录）",
    UNSUPPORTED_LOCATION: "选择本地磁盘位置（不支持 UNC 与设备命名空间）",
    PATH_MISSING: "重新选择位置",
    PATH_UNREADABLE: "检查目标位置权限，或以普通权限另选位置",
    PATH_UNRESOLVABLE: "重新选择位置",
    SNAPSHOT_UNKNOWN: "重新分析或手动刷新当前槽位后再导出",
    SNAPSHOT_MISMATCH: "关闭导出并重新打开（导出必须对应当前槽位）",
    LINK_LOCATION_UNKNOWN: "手动刷新当前槽位后重试",
    LINK_SOURCE_UNKNOWN: "按当前报告重新选择来源",
    LINK_URL_UNSUPPORTED: "该来源仅有书面记录；可改为复制可用线索",
    LINK_OPEN_FAILED: "可重试，或复制已核验链接后手动打开",
  };
  return hints[error.code] ?? null;
}
