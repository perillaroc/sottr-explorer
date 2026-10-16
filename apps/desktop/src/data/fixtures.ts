// 契约夹具导入（SX-015 模拟数据源）。
//
// 这些 JSON 由 `pnpm contract:validate` 与 Rust 契约测试保证与 schema 一致；
// TypeScript 对 JSON 字面量的枚举只会推断为 string，因此这里做一次类型断言，
// 不复制夹具内容（保持夹具唯一来源）。模拟数据全部在界面上标注“模拟数据”。
import degradedJson from "../../../../fixtures/contract/valid/report.degraded.json";
import nonzeroJson from "../../../../fixtures/contract/valid/report.nonzero-group.json";
import staleEnvelopeJson from "../../../../fixtures/contract/valid/desktop-result.stale.json";
import cancelledEnvelopeJson from "../../../../fixtures/contract/valid/desktop-result.cancelled.json";
import metadataFixtureJson from "../../../../fixtures/contract/valid/metadata.valid.json";
import type { Report } from "../contract/report";
import type { DesktopResult } from "../contract/desktop-result";
import type { MetadataDataset } from "../contract/metadata";

/** 降级夹具：表不完整、计数待核对、参考差集不可用（SX-003）。 */
export const degradedReport = degradedJson as unknown as Report;

/** 非零计数夹具：圣胡安 3/8 → general_reference（撤销整组检查）。 */
export const nonzeroReport = nonzeroJson as unknown as Report;

/** 旧快照信封夹具（stale + 刷新错误 + 旧报告）。 */
export const staleEnvelopeFixture = staleEnvelopeJson as unknown as DesktopResult;

/** 取消信封夹具（freshness=none、report/error 均为 null）。 */
export const cancelledEnvelopeFixture = cancelledEnvelopeJson as unknown as DesktopResult;

/** 夹单元数据：用于演示“有来源的本地化名称/人工简名”的显示路径。 */
export const metadataFixture = metadataFixtureJson as unknown as MetadataDataset;
