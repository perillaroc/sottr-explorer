// 模拟输入清单（SX-015）：候选目录与候选文件的演示数据。
//
// 这些路径、时间与大小均为模拟数据（用户名使用“演示用户”），不代表任何真实用户
// 电脑；界面在模拟模式显著标注“模拟数据”。字段形状与 SX-014 命令返回保持一致
// （见 `source.ts` 的 DTO 定义），便于 SX-016 用真实命令替换数据源时无需改界面。
import type {
  CandidateFileDto,
  CandidatesOutcomeDto,
  DiscoverOutcomeDto,
} from "./source";

export type { CandidateFileDto, CandidatesOutcomeDto, DiscoverOutcomeDto };

const documents = "C:\\Users\\演示用户\\Documents";
const fallbackDocuments = "D:\\Games\\Documents";
export const MOCK_ROOT_A = `${documents}\\Shadow of the Tomb Raider`;
export const MOCK_ROOT_B = `${fallbackDocuments}\\Shadow of the Tomb Raider`;
/** 模拟文件系统已知的 Documents 目录（导出目标父级校验用）。 */
export const MOCK_DOCUMENTS = documents;
export const MOCK_FALLBACK_DOCUMENTS = fallbackDocuments;

const gameDirName = "Shadow of the Tomb Raider";

function millis(iso: string): number {
  return Date.parse(iso);
}

export const mockDiscovery: DiscoverOutcomeDto = {
  candidates: [
    {
      path: documents,
      source: "known_folder",
      source_label: "Windows 文档位置（Known Folder）",
      status: "game_found",
      detail: null,
    },
    {
      path: fallbackDocuments,
      source: "user_profile",
      source_label: "%USERPROFILE%\\Documents（回退候选）",
      status: "game_found",
      detail: null,
    },
    {
      path: "E:\\Games\\Documents",
      source: "user_profile",
      source_label: "%USERPROFILE%\\Documents（回退候选）",
      status: "missing",
      detail: null,
    },
    {
      path: `${documents}\\受限备份\\Documents`,
      source: "user_profile",
      source_label: "%USERPROFILE%\\Documents（回退候选）",
      status: "unreadable",
      detail: "拒绝访问（不会提权或修改权限；请用文件选择器手动选择可读位置）",
    },
  ],
  game_roots: [MOCK_ROOT_A, MOCK_ROOT_B],
};

function candidate(
  root: string,
  name: string,
  size: number,
  modified: string,
  note: string | null = null,
): CandidateFileDto {
  return {
    name,
    path: `${root}\\${name}`,
    size_bytes: size,
    modified_millis: millis(modified),
    matches_observed_names: name === "profile.dat" || /^save\d+\.dat$/i.test(name),
    status: "ready",
    detail: null,
    note,
  };
}

/** 每个候选根目录的文件清单（模拟）。 */
export function mockCandidates(root: string): CandidatesOutcomeDto | null {
  if (root === MOCK_ROOT_A) {
    return {
      root,
      files: [
        candidate(root, "save1.dat", 597_590, "2026-10-10T21:14:05+08:00", "组合演示：完整分析、刷新失败一次、再刷新成功"),
        candidate(root, "save202.dat", 597_590, "2026-10-10T21:14:05+08:00", "内容与 save1.dat 相同（不自动合并或替用户选定）"),
        candidate(root, "profile.dat", 580, "2026-10-10T21:14:00+08:00", "资料文件：只做容器观察，不进入战役解释"),
        candidate(root, "challenge_tomb.dat", 1_310_720, "2026-09-30T18:00:00+08:00", "演示分析失败（不是有效的 zlib 流）"),
      ],
      skipped: [
        {
          path: `${root}\\collections_backup`,
          reason: "link_or_reparse_point",
          detail: null,
        },
      ],
      depth_limited_dirs: [],
    };
  }
  if (root === MOCK_ROOT_B) {
    return {
      root,
      files: [
        candidate(root, "save204.dat", 594_970, "2026-10-08T18:02:31+08:00", "早期样本：圣胡安贮藏点计数 3/8（非零组）"),
        candidate(root, "save206.dat", 594_970, "2026-10-08T18:02:31+08:00", "表恢复不完整（异常 type28）：关闭缺席推断"),
        candidate(root, "save_locked.dat", 610_000, "2026-10-11T17:00:00+08:00", "演示首次被占用，重试成功"),
      ],
      skipped: [],
      depth_limited_dirs: [],
    };
  }
  return null;
}

/** 模拟“手动选择文件/文件夹”的返回值（浏览器演示固定返回第一个候选）。 */
export const mockPickedFile = `${MOCK_ROOT_A}\\save1.dat`;
export const mockPickedDirectory = MOCK_ROOT_A;
/** 模拟“导出目标位置”：Documents 根目录（目标为新子目录，不覆盖任何内容）。 */
export const mockPickedExportParent = documents;

/** 供界面显示的模拟说明：候选目录里发生了什么。 */
export const mockRootLabels: Record<string, string> = {
  [MOCK_ROOT_A]: `Known Folder Documents → ${gameDirName}`,
  [MOCK_ROOT_B]: `Documents 回退候选 → ${gameDirName}`,
};
