// 真实数据源命令映射测试（SX-016）：断言 `TauriDataSource` 发出的命令名与参数键
// 与 SX-014 的 IPC 约定一致（camelCase 键 → Rust snake_case 参数），错误归一化，
// 以及取消命令的尽力而为语义。真实后端行为由 Rust IPC 集成测试覆盖。
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { TauriDataSource } from "../src/data/tauri";
import { toCommandError } from "../src/data/errors";

describe("TauriDataSource：命令映射", () => {
  const source = new TauriDataSource();

  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
  });

  it("发现与候选：命令名与参数键", async () => {
    await source.discover();
    expect(invokeMock).toHaveBeenCalledWith("discover_saves");

    invokeMock.mockResolvedValueOnce({ root: "x", files: [], skipped: [], depth_limited_dirs: [] });
    await source.listCandidates("C:\\Saves");
    expect(invokeMock).toHaveBeenCalledWith("list_candidates", { rootPath: "C:\\Saves" });
  });

  it("选择器：用途映射到 file/directory", async () => {
    await source.pick("save-file");
    expect(invokeMock).toHaveBeenCalledWith("select_input", { kind: "file" });
    await source.pick("save-directory");
    expect(invokeMock).toHaveBeenCalledWith("select_input", { kind: "directory" });
    await source.pick("export-directory");
    expect(invokeMock).toHaveBeenCalledWith("select_input", { kind: "directory" });
  });

  it("分析：requestId/inputPath/context 与请求字段一致", async () => {
    invokeMock.mockResolvedValueOnce({ schema_version: "1.0.0" });
    await source.analyze({ requestId: "req-1", path: "C:\\Saves\\save1.dat", purpose: "campaign" });
    expect(invokeMock).toHaveBeenCalledWith("analyze_file", {
      requestId: "req-1",
      inputPath: "C:\\Saves\\save1.dat",
      context: "campaign",
    });

    invokeMock.mockResolvedValueOnce({ schema_version: "1.0.0" });
    await source.analyze({ requestId: "req-2", path: "C:\\Saves\\save2.dat", purpose: "unknown" });
    expect(invokeMock).toHaveBeenCalledWith("analyze_file", {
      requestId: "req-2",
      inputPath: "C:\\Saves\\save2.dat",
      context: "unknown",
    });
  });

  it("取消：幂等且吞掉命令错误（不产生未处理拒绝）", async () => {
    invokeMock.mockRejectedValueOnce({ code: "INVALID_PARAMS", message: "未知请求" });
    await expect(source.cancel("req-done")).resolves.toBeUndefined();
    expect(invokeMock).toHaveBeenCalledWith("cancel_analysis", { requestId: "req-done" });
  });

  it("导出：snapshotId/inputRef/outputDir/format/share", async () => {
    invokeMock.mockResolvedValueOnce({
      snapshot_id: "snap-1",
      input_ref: "save1.dat",
      freshness: "current",
      last_refresh_error_code: null,
      share: true,
      artifacts: [],
    });
    await source.exportReport({
      snapshotId: "snap-1",
      inputRef: "save1.dat",
      outputDir: "C:\\out\\new-dir",
      format: "both",
      share: true,
    });
    expect(invokeMock).toHaveBeenCalledWith("export_report", {
      snapshotId: "snap-1",
      inputRef: "save1.dat",
      outputDir: "C:\\out\\new-dir",
      format: "both",
      share: true,
    });
  });

  it("攻略链接：解析与打开分别映射到 resolve/open 命令", async () => {
    invokeMock.mockResolvedValue({ url: "https://example.com/" });
    await source.resolveGuideLink({
      snapshotId: "snap-1",
      locationId: "loc:x",
      sourceRef: "src:stella-san-juan",
    });
    expect(invokeMock).toHaveBeenCalledWith("resolve_guide_link", {
      snapshotId: "snap-1",
      locationId: "loc:x",
      sourceRef: "src:stella-san-juan",
    });

    invokeMock.mockClear();
    invokeMock.mockResolvedValue({ url: "https://example.com/" });
    await source.openGuideLink({
      snapshotId: "snap-1",
      locationId: "loc:x",
      sourceRef: "src:stella-san-juan",
    });
    expect(invokeMock).toHaveBeenCalledWith("open_guide_link", {
      snapshotId: "snap-1",
      locationId: "loc:x",
      sourceRef: "src:stella-san-juan",
    });
  });

  it("命令错误原样拒绝（{code,message}）；归一化后保留稳定错误码", async () => {
    invokeMock.mockRejectedValueOnce({ code: "LINK_OPEN_FAILED", message: "系统浏览器打开失败" });
    await expect(
      source.openGuideLink({ snapshotId: "snap-1", locationId: "loc:x", sourceRef: "src:a" }),
    ).rejects.toMatchObject({ code: "LINK_OPEN_FAILED" });

    expect(toCommandError({ code: "LINK_OPEN_FAILED", message: "系统浏览器打开失败" })).toEqual({
      code: "LINK_OPEN_FAILED",
      message: "系统浏览器打开失败",
    });
  });
});
