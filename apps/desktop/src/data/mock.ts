// 模拟数据源（SX-015 起；SX-016 补齐导出与攻略链接）：把契约夹具与演示输入
// 伪装成 SX-014 命令的返回。
//
// - 所有返回都是模拟数据，界面在模拟模式显著标注；不接触文件系统、不访问网络；
// - 场景覆盖：完整、非零组、表不完整、资料文件、被占用后重试成功、分析失败、
//   刷新失败（stale，携带旧报告）、取消（无结果信封）、导出（新目录/
//   已存在/重叠/非法名称）、链接解析（无 URL 来源）与打开失败（可选开关）；
// - `delayMs` 用于演示“分析中/取消”；测试使用 0 保持确定性；
// - 导出与链接解析错误码与 SX-014 保持一致，便于界面走同一条恢复路径。
import type { DesktopResult } from "../contract/desktop-result";
import { demoMetadata } from "./metadata";
import {
  MOCK_DOCUMENTS,
  MOCK_ONEDRIVE_DOCUMENTS,
  MOCK_ROOT_A,
  MOCK_ROOT_B,
  mockCandidates,
  mockDiscovery,
  mockPickedFile,
  mockPickedDirectory,
  mockPickedExportParent,
} from "./mockInputs";
import {
  buildCompleteReport,
  buildDegradedReport,
  buildNonzeroReport,
  buildProfileReport,
} from "./mockReports";
import type {
  AnalyzeRequest,
  CandidatesOutcomeDto,
  CommandErrorDto,
  DataSource,
  DiscoverOutcomeDto,
  ExportOutcomeDto,
  ExportRequest,
  GuideLinkDto,
  GuideLinkRequest,
  InputError,
  PickKind,
  Purpose,
} from "./source";

interface FileSeed {
  sha: string;
  size: number;
}

/** 模拟输入摘要：字段形状同真实报告（压缩 SHA-256、压缩字节数）。 */
const FILE_SEEDS: Record<string, FileSeed> = {
  "save1.dat": { sha: "aaa1b6c0d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f80", size: 597_590 },
  "save202.dat": { sha: "aaa1b6c0d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f80", size: 597_590 },
  "save204.dat": { sha: "bbb2c7d1e8f90213a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b", size: 594_970 },
  "save206.dat": { sha: "bbb2c7d1e8f90213a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b", size: 594_970 },
  "profile.dat": { sha: "ccc3d8e2f90a1324b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c", size: 580 },
  "save_locked.dat": { sha: "ddd4e9f30a1b2435c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d", size: 610_000 },
  "challenge_tomb.dat": { sha: "eee5f0a41b2c3546d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e", size: 1_310_720 },
};

const SCHEMA_VERSION = "1.0.0" as const;

function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

function dirname(path: string): string {
  const parts = path.split(/[\\/]/);
  parts.pop();
  return parts.join("\\");
}

function normalizePath(path: string): string {
  return path.replace(/\//g, "\\").replace(/[\\]+$/, "").toLowerCase();
}

function seedOf(path: string): FileSeed {
  return FILE_SEEDS[basename(path)] ?? { sha: "0".repeat(64), size: 0 };
}

/** 模拟文件系统：候选文件所在目录、候选文件本身，以及本次会话创建过的导出目录。 */
const MOCK_DIRECTORIES: string[] = [MOCK_DOCUMENTS, MOCK_ONEDRIVE_DOCUMENTS, MOCK_ROOT_A, MOCK_ROOT_B];
const MOCK_FILES: string[] = [...(mockCandidates(MOCK_ROOT_A)?.files ?? []), ...(mockCandidates(MOCK_ROOT_B)?.files ?? [])].map(
  (file) => file.path,
);

function mockDirExists(path: string): boolean {
  const normalized = normalizePath(path);
  return MOCK_DIRECTORIES.some((dir) => normalizePath(dir) === normalized);
}

function mockFileExists(path: string): boolean {
  const normalized = normalizePath(path);
  return MOCK_FILES.some((file) => normalizePath(file) === normalized);
}

function isWithin(child: string, ancestor: string): boolean {
  const normalizedChild = normalizePath(child);
  const normalizedAncestor = normalizePath(ancestor);
  return normalizedChild === normalizedAncestor || normalizedChild.startsWith(`${normalizedAncestor}\\`);
}

function mockCommandError(code: string, message: string): CommandErrorDto {
  return { code, message };
}

/** 文件名合法性（Windows：非法字符、保留结尾、空名）。 */
function invalidFolderName(name: string): string | null {
  if (name.trim().length === 0) {
    return "目录名不能为空";
  }
  if (/[<>:"/\\|?*\u0000-\u001f]/.test(name)) {
    return "目录名包含 Windows 非法字符（< > : \" / \\ | ? *）";
  }
  if (/[. ]$/.test(name)) {
    return "目录名不能以空格或句点结尾";
  }
  const reserved = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;
  if (reserved.test(name)) {
    return "目录名是 Windows 保留名";
  }
  return null;
}

export interface MockOptions {
  /** 每次命令的演示延迟（毫秒）；测试传 0。 */
  delayMs?: number;
  /** 首次用途声明（未选时由界面默认）。 */
  defaultPurpose?: Purpose;
  /** 演示“系统浏览器打开失败”（仅模拟；真实失败路径由后端命令返回）。 */
  failGuideOpen?: boolean;
}

/** 已提交的模拟快照（导出与链接解析必须绑定同一快照）。 */
interface StoredSnapshot {
  result: DesktopResult;
  inputPath: string;
}

export class MockDataSource implements DataSource {
  readonly kind = "mock" as const;

  private readonly delayMs: number;
  private readonly failGuideOpen: boolean;
  private readonly attempts = new Map<string, number>();
  private readonly lastResults = new Map<string, DesktopResult>();
  private readonly snapshots = new Map<string, StoredSnapshot>();
  private readonly createdExportDirs = new Set<string>();
  private readonly cancelled = new Set<string>();
  /** 已“打开”的链接（供测试断言不加载 WebView）。 */
  readonly openedUrls: string[] = [];

  constructor(options: MockOptions = {}) {
    this.delayMs = options.delayMs ?? 0;
    this.failGuideOpen = options.failGuideOpen ?? false;
  }

  private wait(): Promise<void> {
    return this.delayMs > 0 ? new Promise((resolve) => setTimeout(resolve, this.delayMs)) : Promise.resolve();
  }

  async discover(): Promise<DiscoverOutcomeDto> {
    await this.wait();
    return mockDiscovery;
  }

  async listCandidates(root: string): Promise<CandidatesOutcomeDto> {
    await this.wait();
    return (
      mockCandidates(root) ?? {
        root,
        files: [],
        skipped: [],
        depth_limited_dirs: [],
      }
    );
  }

  async pick(kind: PickKind): Promise<string | null> {
    await this.wait();
    switch (kind) {
      case "save-file":
        return mockPickedFile;
      case "save-directory":
        return mockPickedDirectory;
      default:
        return mockPickedExportParent;
    }
  }

  async analyze(request: AnalyzeRequest): Promise<DesktopResult> {
    const attempt = (this.attempts.get(request.path) ?? 0) + 1;
    this.attempts.set(request.path, attempt);
    await this.wait();
    if (this.cancelled.has(request.requestId)) {
      return this.cancelledResult(request);
    }
    const result = this.scenario(request, attempt);
    if (result.report && result.snapshot_id) {
      if (result.freshness === "current") {
        this.lastResults.set(request.path, result);
        this.snapshots.set(result.snapshot_id, { result, inputPath: request.path });
      } else if (result.freshness === "stale") {
        // 与 SX-014 一致：失败刷新把既有快照原地标记为 stale（身份不变）。
        this.snapshots.set(result.snapshot_id, { result, inputPath: request.path });
      }
    }
    return result;
  }

  cancel(requestId: string): void {
    this.cancelled.add(requestId);
  }

  /**
   * 模拟导出：只做与 SX-014 一致的路径与快照校验（新目录、非重叠、名称合法），
   * 返回约定产物路径；不落盘、不覆盖任何内容。同一目标第二次导出报
   * `OUTPUT_EXISTS`（与真实行为一致，便于演示恢复路径）。
   */
  async exportReport(request: ExportRequest): Promise<ExportOutcomeDto> {
    await this.wait();
    const stored = this.snapshots.get(request.snapshotId);
    if (!stored) {
      throw mockCommandError("SNAPSHOT_UNKNOWN", "模拟：当前报告的快照已不可用，请重新分析");
    }
    if (stored.result.input_ref !== request.inputRef) {
      throw mockCommandError(
        "SNAPSHOT_MISMATCH",
        `模拟：导出请求与当前槽位不一致（快照属于 ${stored.result.input_ref}）`,
      );
    }
    const output = request.outputDir.replace(/\/+$/, "");
    const folder = basename(output);
    const nameProblem = invalidFolderName(folder);
    if (nameProblem) {
      throw mockCommandError("OUTPUT_INVALID_TARGET", `模拟：${nameProblem}`);
    }
    const parent = dirname(output);
    if (mockDirExists(output)) {
      throw mockCommandError("OUTPUT_EXISTS", `模拟：导出目标目录已存在：${output}`);
    }
    if (mockFileExists(output)) {
      throw mockCommandError("OUTPUT_EXISTS", `模拟：导出目标已被同名文件占用：${output}`);
    }
    if (!mockDirExists(parent)) {
      if (mockFileExists(parent)) {
        throw mockCommandError("OUTPUT_PARENT_NOT_DIRECTORY", `模拟：目标父级不是目录：${parent}`);
      }
      throw mockCommandError("PATH_MISSING", `模拟：目标父级不存在：${parent}`);
    }
    const inputDir = dirname(stored.inputPath);
    if (isWithin(output, inputDir) || isWithin(inputDir, output)) {
      throw mockCommandError(
        "INPUT_OUTPUT_OVERLAP",
        `模拟：导出目标不能与存档目录重叠（${output} 与 ${inputDir}）`,
      );
    }

    this.createdExportDirs.add(normalizePath(output));
    MOCK_DIRECTORIES.push(output);
    const artifacts: { name: string; path: string }[] = [];
    if (request.format === "json" || request.format === "both") {
      artifacts.push({ name: "report.json", path: `${output}\\report.json` });
    }
    if (request.format === "csv" || request.format === "both") {
      artifacts.push({ name: "report.csv", path: `${output}\\report.csv` });
    }
    return {
      snapshot_id: stored.result.snapshot_id ?? request.snapshotId,
      input_ref: stored.result.input_ref,
      freshness: stored.result.freshness,
      last_refresh_error_code: stored.result.last_refresh_error?.code ?? null,
      share: request.share,
      artifacts,
    };
  }

  /** 模拟解析：返回元数据中的已校验 URL；错误码与 SX-014 一致。 */
  async resolveGuideLink(request: GuideLinkRequest): Promise<GuideLinkDto> {
    await this.wait();
    const stored = this.snapshots.get(request.snapshotId);
    if (!stored || !stored.result.report) {
      throw mockCommandError("SNAPSHOT_UNKNOWN", "模拟：当前报告的快照已不可用，请重新分析");
    }
    const location = stored.result.report.locations.find(
      (entry) => entry.location_id === request.locationId,
    );
    if (!location) {
      throw mockCommandError("LINK_LOCATION_UNKNOWN", `模拟：当前报告没有定位线索 ${request.locationId}`);
    }
    if (!location.source_refs.includes(request.sourceRef)) {
      throw mockCommandError(
        "LINK_SOURCE_UNKNOWN",
        `模拟：来源 ${request.sourceRef} 不属于该定位线索`,
      );
    }
    const source = demoMetadata.sources[request.sourceRef] ?? null;
    const url = source?.url ?? null;
    if (!url || !/^https?:\/\//i.test(url)) {
      throw mockCommandError(
        "LINK_URL_UNSUPPORTED",
        `模拟：来源 ${request.sourceRef} 没有可打开的 http/https 链接`,
      );
    }
    return {
      snapshot_id: stored.result.snapshot_id ?? request.snapshotId,
      input_ref: stored.result.input_ref,
      location_id: location.location_id,
      source_ref: request.sourceRef,
      url,
      freshness: stored.result.freshness,
      action_kind: location.group_actionability?.kind ?? null,
      action_reason_codes: location.group_actionability?.reason_codes ?? [],
    };
  }

  /** 模拟打开：记录 URL 而不加载 WebView；可配置为失败以演示恢复入口。 */
  async openGuideLink(request: GuideLinkRequest): Promise<GuideLinkDto> {
    const link = await this.resolveGuideLink(request);
    await this.wait();
    if (this.failGuideOpen) {
      throw mockCommandError(
        "LINK_OPEN_FAILED",
        "模拟：系统浏览器打开失败（LINK_OPEN_FAILED 演示开关）",
      );
    }
    this.openedUrls.push(link.url);
    return link;
  }

  /** 已提交结果（供界面与测试断言“切槽不串用报告”）。 */
  committed(path: string): DesktopResult | null {
    return this.lastResults.get(path) ?? null;
  }

  /** 模拟报告的输入摘要（压缩 SHA-256 与字节数）。 */
  private reportSeed(path: string): { compressedSha256: string; compressedBytes: number } {
    const seed = seedOf(path);
    return { compressedSha256: seed.sha, compressedBytes: seed.size };
  }

  // -------------------------------------------------------------------------
  // 场景
  // -------------------------------------------------------------------------

  private scenario(request: AnalyzeRequest, attempt: number): DesktopResult {
    const name = basename(request.path);
    const seed = this.reportSeed(request.path);
    switch (name) {
      case "profile.dat":
        return this.currentResult(request, buildProfileReport(seed));
      case "save204.dat":
        return this.currentResult(request, buildNonzeroReport(seed));
      case "save206.dat":
        return this.currentResult(request, buildDegradedReport(seed));
      case "challenge_tomb.dat":
        return this.errorResult(
          request,
          mockError(
            "DECOMPRESSION_FAILED",
            "不是有效的 zlib 流（模拟：挑战古墓专用文件或损坏文件）",
            false,
          ),
        );
      case "save_locked.dat":
        if (attempt === 1) {
          return this.errorResult(
            request,
            mockError(
              "INPUT_UNREADABLE",
              "文件可能正在保存或被其他程序占用：save_locked.dat；请稍后重试（模拟数据）",
              true,
            ),
          );
        }
        return this.currentResult(request, buildCompleteReport(seed));
      case "save202.dat":
      case "save1.dat": {
        const refreshFailure = name === "save1.dat" && attempt === 2;
        if (refreshFailure) {
          return this.staleResult(request);
        }
        return this.currentResult(request, buildCompleteReport(seed));
      }
      default:
        return this.currentResult(request, buildCompleteReport(seed));
    }
  }

  private snapshotId(): string {
    return `snap-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`;
  }

  private currentResult(
    request: AnalyzeRequest,
    report: DesktopResult["report"],
  ): DesktopResult {
    return {
      schema_version: SCHEMA_VERSION,
      request_id: request.requestId,
      input_ref: basename(request.path),
      snapshot_id: this.snapshotId(),
      snapshot_time: new Date().toISOString(),
      freshness: "current",
      last_refresh_error: null,
      report,
      error: null,
    };
  }

  private errorResult(request: AnalyzeRequest, error: InputError): DesktopResult {
    return {
      schema_version: SCHEMA_VERSION,
      request_id: request.requestId,
      input_ref: basename(request.path),
      snapshot_id: null,
      snapshot_time: null,
      freshness: "none",
      last_refresh_error: null,
      report: null,
      error,
    };
  }

  private cancelledResult(request: AnalyzeRequest): DesktopResult {
    return {
      schema_version: SCHEMA_VERSION,
      request_id: request.requestId,
      input_ref: basename(request.path),
      snapshot_id: null,
      snapshot_time: null,
      freshness: "none",
      last_refresh_error: null,
      report: null,
      error: null,
    };
  }

  /**
   * 刷新失败：保留旧报告并标记 stale + 刷新错误（形状同 SX-014 后端；
   * 旧快照身份与内容摘要来自上次成功结果）。
   */
  private staleResult(request: AnalyzeRequest): DesktopResult {
    const previous = this.lastResults.get(request.path);
    const error = mockError(
      "INPUT_CHANGED",
      "读取期间文件已变化（模拟：文件正在保存）；保留旧快照并标记过期",
      true,
    );
    if (!previous || !previous.report) {
      return this.errorResult(request, error);
    }
    return {
      schema_version: SCHEMA_VERSION,
      request_id: request.requestId,
      input_ref: previous.input_ref,
      snapshot_id: previous.snapshot_id,
      snapshot_time: previous.snapshot_time,
      freshness: "stale",
      last_refresh_error: error,
      report: previous.report,
      error,
    };
  }
}

function mockError(code: string, message: string, retryable: boolean): InputError {
  return { code, message, retryable };
}

/** 浏览器/开发环境使用的模拟数据源（带演示延迟）。 */
export function createDemoSource(options: { failGuideOpen?: boolean } = {}): MockDataSource {
  return new MockDataSource({ delayMs: 650, failGuideOpen: options.failGuideOpen });
}

export { MOCK_ROOT_A, MOCK_ROOT_B, mockPickedFile, mockPickedDirectory, mockPickedExportParent };
