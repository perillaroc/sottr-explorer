// 命令错误的归一化与恢复提示（SX-016）：Tauri `invoke` 拒绝时会给出命令序列化
// 的错误负载（`{ code, message }`，见 SX-014 `CommandError`）；网络层/未知异常
// 则给出字符串或 Error。界面只依赖 `{ code, message }`，因此统一在这里收敛。
//
// 说明：后端 `message` 已是面向用户的说明（不泄露私人路径之外的内部信息），
// 这里只按稳定错误码追加“下一步可做什么”的恢复提示，不替换后端原因。
import { t, type MessageKey } from "../app/i18n";
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
  return { code: CODE_UNEXPECTED, message: t("errors.command-failed-without-a-readable-reason") };
}

/** 稳定错误码 → 恢复建议（没有对应建议返回 null）。 */
export function recoveryHint(error: CommandErrorDto): string | null {
  const hints: Record<string, MessageKey> = {
    OUTPUT_EXISTS: "recoveryHint.OUTPUT_EXISTS",
    OUTPUT_PARENT_NOT_DIRECTORY: "recoveryHint.OUTPUT_PARENT_NOT_DIRECTORY",
    OUTPUT_INVALID_TARGET: "recoveryHint.OUTPUT_INVALID_TARGET",
    OUTPUT_LINK_REPLACED: "recoveryHint.OUTPUT_LINK_REPLACED",
    OUTPUT_WRITE_FAILED: "recoveryHint.OUTPUT_WRITE_FAILED",
    OUTPUT_COMMIT_FAILED: "recoveryHint.OUTPUT_COMMIT_FAILED",
    INPUT_OUTPUT_OVERLAP: "recoveryHint.INPUT_OUTPUT_OVERLAP",
    UNSUPPORTED_LOCATION: "recoveryHint.UNSUPPORTED_LOCATION",
    PATH_MISSING: "recoveryHint.PATH_MISSING",
    PATH_UNREADABLE: "recoveryHint.PATH_UNREADABLE",
    PATH_UNRESOLVABLE: "recoveryHint.PATH_UNRESOLVABLE",
    SNAPSHOT_UNKNOWN: "recoveryHint.SNAPSHOT_UNKNOWN",
    SNAPSHOT_MISMATCH: "recoveryHint.SNAPSHOT_MISMATCH",
    LINK_LOCATION_UNKNOWN: "recoveryHint.LINK_LOCATION_UNKNOWN",
    LINK_SOURCE_UNKNOWN: "recoveryHint.LINK_SOURCE_UNKNOWN",
    LINK_URL_UNSUPPORTED: "recoveryHint.LINK_URL_UNSUPPORTED",
    LINK_OPEN_FAILED: "recoveryHint.LINK_OPEN_FAILED",
  };
  const hint = hints[error.code];
  return hint ? t(hint) : null;
}
