// 外部参考链接（2026-10-17 详情重设计阶段三）：搜索地址合成与图文/视频判定。
//
// 只做纯字符串拼装，不读 store、不发起命令：详情面板与 store 各自取用。
// 查询词带上区域名以提高命中率（区域名由调用方从计划里取，见 `app/collection.ts`）。
import type { PlanItem } from "../data/atlas";
import type { SearchEngine } from "./prefs";

/** 游戏名中文常用写法，用于搜索词。 */
const GAME_TERM = "古墓丽影暗影";

/** 搜索引擎表（设置页三选一）；`{q}` 为查询词占位符。 */
export const SEARCH_ENGINES = {
  bing: { label: "Bing", template: "https://www.bing.com/search?q={q}" },
  google: { label: "Google", template: "https://www.google.com/search?q={q}" },
  baidu: { label: "百度", template: "https://www.baidu.com/s?wd={q}" },
} as const satisfies Record<SearchEngine, { label: string; template: string }>;

/** 参考链接的载体：图文页或视频页。 */
export type LinkMedium = "text" | "video";

/** 视频站点主机名；命中子域也算（如 `www.bilibili.com`、`m.youtube.com`）。 */
const VIDEO_HOSTS = [
  "bilibili.com",
  "youtube.com",
  "youtu.be",
  "youku.com",
  "iqiyi.com",
  "v.qq.com",
  "twitch.tv",
  "douyin.com",
];

/**
 * 按主机名猜图文还是视频——**临时启发式**：数据层的 `Source` 还没有 `medium`
 * 字段（补数据见设计文档 9.1「中文图文攻略与视频攻略来源探索」）。届时本函数
 * 直接删除，改读显式字段；在此之前只有它决定直链按钮归到「攻略页」还是「视频页」。
 */
export function mediumOfUrl(url: string): LinkMedium {
  let host: string;
  try {
    host = new URL(url).hostname.toLowerCase();
  } catch {
    return "text";
  }
  return VIDEO_HOSTS.some((known) => host === known || host.endsWith(`.${known}`)) ? "video" : "text";
}

/** 把查询词编码后填进引擎模板。 */
export function buildSearchUrl(engine: SearchEngine, query: string): string {
  return SEARCH_ENGINES[engine].template.replace("{q}", encodeURIComponent(query));
}

/** 图文攻略搜索词：条目名（中英）+ 区域名 + 游戏名 + 攻略。 */
export function textSearchQuery(item: PlanItem, regionName: string): string {
  return joinQuery([item.display_name, item.name_en, regionName, GAME_TERM, "攻略"]);
}

/** 视频攻略搜索词：条目名 + 区域名 + 游戏名 + 视频攻略。 */
export function videoSearchQuery(item: PlanItem, regionName: string): string {
  return joinQuery([item.display_name, regionName, GAME_TERM, "视频攻略"]);
}

/** 拼查询词：丢掉缺失项（条目未必有英文名），空格分隔。 */
function joinQuery(parts: (string | undefined)[]): string {
  return parts.filter((part): part is string => Boolean(part?.trim())).join(" ");
}
