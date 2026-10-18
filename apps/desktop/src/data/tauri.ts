// 真实数据源（SX-016）：把界面请求映射到 SX-014 的最小权限 Tauri 命令。
//
// - 参数键使用 camelCase（Tauri 2 默认把 JS 参数字段映射到 Rust snake_case 参数，
//   见 SX-014 IPC 测试的实际调用形态）；命令返回保持契约的 snake_case；
// - 命令级错误（`{ code, message }`）拒绝 Promise，由上层 `toCommandError` 归一化；
// - 不在此处做任何领域判断：能力、门禁、定位精度全部读取返回的报告结论；
// - 前端没有 opener/dialog/shell/fs 权限：外部链接与原生选择器都经命令内部调用。
import { invoke } from "@tauri-apps/api/core";
import type { DesktopResult } from "../contract/desktop-result";
import type {
  AnalyzeRequest,
  CandidatesOutcomeDto,
  DataSource,
  DiscoverOutcomeDto,
  ExportOutcomeDto,
  ExportRequest,
  GuideLinkDto,
  GuideLinkRequest,
  PickKind,
} from "./source";

/** 是否运行在 Tauri（WebView）内；浏览器开发/测试环境返回 false。 */
export function isTauriRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

export class TauriDataSource implements DataSource {
  readonly kind = "tauri" as const;

  discover(): Promise<DiscoverOutcomeDto> {
    return invoke<DiscoverOutcomeDto>("discover_saves");
  }

  listCandidates(root: string): Promise<CandidatesOutcomeDto> {
    return invoke<CandidatesOutcomeDto>("list_candidates", { rootPath: root });
  }

  pick(kind: PickKind): Promise<string | null> {
    // 真实命令只区分文件/目录；用途（存档/导出位置）由界面决定落点。
    const nativeKind = kind === "save-file" ? "file" : "directory";
    return invoke<string | null>("select_input", { kind: nativeKind });
  }

  analyze(request: AnalyzeRequest): Promise<DesktopResult> {
    return invoke<DesktopResult>("analyze_file", {
      requestId: request.requestId,
      inputPath: request.path,
      context: request.purpose,
    });
  }

  cancel(requestId: string): Promise<void> {
    // 取消是尽力而为且幂等：命令错误（如请求已完成）不影响界面状态机。
    return invoke("cancel_analysis", { requestId })
      .then(() => undefined)
      .catch(() => undefined);
  }

  exportReport(request: ExportRequest): Promise<ExportOutcomeDto> {
    return invoke<ExportOutcomeDto>("export_report", {
      snapshotId: request.snapshotId,
      inputRef: request.inputRef,
      outputDir: request.outputDir,
      format: request.format,
      share: request.share,
    });
  }

  resolveGuideLink(request: GuideLinkRequest): Promise<GuideLinkDto> {
    return invoke<GuideLinkDto>("resolve_guide_link", {
      snapshotId: request.snapshotId,
      locationId: request.locationId,
      sourceRef: request.sourceRef,
    });
  }

  openGuideLink(request: GuideLinkRequest): Promise<GuideLinkDto> {
    return invoke<GuideLinkDto>("open_guide_link", {
      snapshotId: request.snapshotId,
      locationId: request.locationId,
      sourceRef: request.sourceRef,
    });
  }

  openExternalUrl(url: string): Promise<void> {
    // 合成地址不经快照：命令只做 http/https 白名单校验后交给系统浏览器。
    return invoke("open_external_url", { url }).then(() => undefined);
  }
}
