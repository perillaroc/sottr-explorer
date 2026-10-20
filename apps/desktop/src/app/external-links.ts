// 外部参考链接（2026-10-17 详情重设计阶段三）：搜索地址合成。
//
// 只做纯字符串拼装，不读 store、不发起命令：详情面板与 store 各自取用。
// 查询词带上区域名以提高命中率（区域名由调用方从计划里取，见 `app/collection.ts`）。
import type { PlanItem } from "../data/atlas";
import { t } from "./i18n";
import type { SearchEngine } from "./prefs";

/** 游戏名常用写法（随界面语言切换搜索词）。 */
function gameTerm(): string {
  return t("古墓丽影暗影", "Shadow of the Tomb Raider");
}

/** 搜索引擎表（设置页三选一）；`{q}` 为查询词占位符。 */
export const SEARCH_ENGINES = {
  bing: { label: "Bing", template: "https://www.bing.com/search?q={q}" },
  google: { label: "Google", template: "https://www.google.com/search?q={q}" },
  baidu: { label: "Baidu", template: "https://www.baidu.com/s?wd={q}" },
} as const satisfies Record<SearchEngine, { label: string; template: string }>;

/** 参考链接的载体：图文页或视频页。 */
export type LinkMedium = "text" | "video";

/** 把查询词编码后填进引擎模板。 */
export function buildSearchUrl(engine: SearchEngine, query: string): string {
  return SEARCH_ENGINES[engine].template.replace("{q}", encodeURIComponent(query));
}

/** 图文攻略搜索词：条目名（中英）+ 区域名 + 游戏名 + 攻略。 */
export function textSearchQuery(item: PlanItem, regionName: string): string {
  return joinQuery([item.display_name, item.name_en, regionName, gameTerm(), t("攻略", "guide")]);
}

/** 视频攻略搜索词：条目名 + 区域名 + 游戏名 + 视频攻略。 */
export function videoSearchQuery(item: PlanItem, regionName: string): string {
  return joinQuery([item.display_name, regionName, gameTerm(), t("视频攻略", "video walkthrough")]);
}

/** 拼查询词：丢掉缺失项（条目未必有英文名），空格分隔。 */
function joinQuery(parts: (string | undefined)[]): string {
  return parts.filter((part): part is string => Boolean(part?.trim())).join(" ");
}
