// 模拟数据源（SX-015）：把契约夹具与演示输入伪装成 SX-014 命令的返回。
//
// - 所有返回都是模拟数据，界面在模拟模式显著标注；不接触文件系统、不访问网络；
// - 场景覆盖：完整、非零组、表不完整、资料文件、被占用后重试成功、分析失败、
//   刷新失败（stale，携带旧报告）、取消（无结果信封）；
// - `delayMs` 用于演示“分析中/取消”；测试使用 0 保持确定性。
import type { DesktopResult } from "../contract/desktop-result";
import {
  MOCK_ROOT_A,
  MOCK_ROOT_B,
  mockCandidates,
  mockDiscovery,
  mockPickedFile,
  mockPickedFolder,
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
  DataSource,
  DiscoverOutcomeDto,
  InputError,
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

function seedOf(path: string): FileSeed {
  return FILE_SEEDS[basename(path)] ?? { sha: "0".repeat(64), size: 0 };
}

export interface MockOptions {
  /** 每次命令的演示延迟（毫秒）；测试传 0。 */
  delayMs?: number;
  /** 首次用途声明（未选时由界面默认）。 */
  defaultPurpose?: Purpose;
}

export class MockDataSource implements DataSource {
  readonly kind = "mock" as const;

  private readonly delayMs: number;
  private readonly attempts = new Map<string, number>();
  private readonly lastResults = new Map<string, DesktopResult>();
  private readonly cancelled = new Set<string>();

  constructor(options: MockOptions = {}) {
    this.delayMs = options.delayMs ?? 0;
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

  async pick(kind: "file" | "directory"): Promise<string | null> {
    await this.wait();
    return kind === "file" ? mockPickedFile : mockPickedFolder;
  }

  async analyze(request: AnalyzeRequest): Promise<DesktopResult> {
    const attempt = (this.attempts.get(request.path) ?? 0) + 1;
    this.attempts.set(request.path, attempt);
    await this.wait();
    if (this.cancelled.has(request.requestId)) {
      return this.cancelledResult(request);
    }
    const result = this.scenario(request, attempt);
    if (result.report && result.freshness === "current") {
      this.lastResults.set(request.path, result);
    }
    return result;
  }

  cancel(requestId: string): void {
    this.cancelled.add(requestId);
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
export function createDemoSource(): MockDataSource {
  return new MockDataSource({ delayMs: 650 });
}

export { MOCK_ROOT_A, MOCK_ROOT_B, mockPickedFile, mockPickedFolder };
