#!/usr/bin/env node
/**
 * 文档一致性校验：错误码与术语表。
 *
 * 1. 错误码：从 Rust 源码提取稳定码（常量与已知前缀的字面量），
 *    与 docs/reference/errors.md 中列出的码比较；文档可用 `PREFIX_*` 通配。
 * 2. 术语表：从 JSON Schema 的领域枚举抽取取值，要求每一个都在
 *    docs/reference/glossary.md 中出现。
 *
 * 退出码 0 表示一致；1 表示存在缺失或未知码/术语。构建时执行。
 */
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const repo = path.resolve(here, '../..')
const docs = path.join(repo, 'docs')

const read = (p) => fs.readFileSync(p, 'utf8')

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(full, out)
    else if (entry.isFile()) out.push(full)
  }
  return out
}

/* ------------------------------------------------------------------ */
/* 错误码                                                              */
/* ------------------------------------------------------------------ */

const rustRoots = [
  ...fs
    .readdirSync(path.join(repo, 'crates'), { withFileTypes: true })
    .filter((e) => e.isDirectory() && e.name.startsWith('sottr-'))
    .map((e) => path.join(repo, 'crates', e.name, 'src')),
  path.join(repo, 'apps', 'desktop', 'src-tauri', 'src'),
].filter((p) => fs.existsSync(p))

const rustFiles = rustRoots.flatMap((root) => walk(root).filter((f) => f.endsWith('.rs')))

const codeConst =
  /(?:pub\s+)?const\s+[A-Z0-9_]+\s*:\s*&str\s*=\s*"([A-Z][A-Z0-9_]+)"/g
// 诊断码多为函数内字面量；只收录项目约定的前缀，避免把普通常量当成错误码。
const codeLiteral =
  /"((?:MODE|BUILD_UNKNOWN|DLC_UNKNOWN|CONTAINER|COUNTER|METADATA|UNSUPPORTED|LOCATION|INVENTORY|GROUP|REFERENCE|SNAPSHOT|CONTEXT|COMPLETE|ZERO|INPUT|DECOMPRESSION|RESOURCE|BASEMAP|CATALOG|DISCOVERY|PATH|OUTPUT|LINK)_[A-Z0-9_]+)"/g

const sourceCodes = new Set()
for (const file of rustFiles) {
  const text = read(file)
  for (const m of text.matchAll(codeConst)) sourceCodes.add(m[1])
  for (const m of text.matchAll(codeLiteral)) sourceCodes.add(m[1])
}

const errorsText = read(path.join(docs, 'reference', 'errors.md'))
const wildcardPrefixes = [...errorsText.matchAll(/`([A-Z][A-Z0-9_]*)\*`/g)].map(
  (m) => m[1],
)
const documentedCodes = new Set(
  [...errorsText.matchAll(/`([A-Z][A-Z0-9_]{2,})`/g)]
    .map((m) => m[1])
    .filter((code) => !code.endsWith('_')),
)

const missingCodes = [...sourceCodes]
  .filter((code) => !documentedCodes.has(code))
  .filter((code) => !wildcardPrefixes.some((prefix) => code.startsWith(prefix)))
  .sort()

const unknownCodes = [...documentedCodes]
  .filter((code) => !sourceCodes.has(code))
  .sort()

/* ------------------------------------------------------------------ */
/* 术语表                                                              */
/* ------------------------------------------------------------------ */

const glossaryText = read(path.join(docs, 'reference', 'glossary.md'))

function schemaEnum(file, defName) {
  const schema = JSON.parse(read(path.join(repo, 'schemas', file)))
  const def = schema.$defs?.[defName]
  if (!def) throw new Error(`schema ${file} 缺少 $defs.${defName}`)
  const values = new Set()
  const walkEnum = (node) => {
    if (Array.isArray(node)) {
      for (const item of node) walkEnum(item)
    } else if (node && typeof node === 'object') {
      if (Array.isArray(node.enum)) {
        for (const value of node.enum) {
          if (typeof value === 'string') values.add(value)
        }
      }
      for (const value of Object.values(node)) walkEnum(value)
    }
  }
  walkEnum(def)
  return values
}

const enumGroups = [
  ['SupportStatus', schemaEnum('report.schema.json', 'SupportStatus')],
  ['Mode', schemaEnum('report.schema.json', 'Mode')],
  ['ContextSource', schemaEnum('report.schema.json', 'ContextSource')],
  ['ObservationKind', schemaEnum('report.schema.json', 'ObservationKind')],
  ['AssessmentKind', schemaEnum('report.schema.json', 'AssessmentKind')],
  ['EvidenceLevel', schemaEnum('report.schema.json', 'EvidenceLevel')],
  ['Severity', schemaEnum('report.schema.json', 'Severity')],
  ['DiagnosticScope', schemaEnum('report.schema.json', 'DiagnosticScope')],
  ['InventoryTable', schemaEnum('report.schema.json', 'InventoryTable')],
  ['LocationPrecision', schemaEnum('report.schema.json', 'LocationPrecision')],
  ['MappingKind', schemaEnum('report.schema.json', 'MappingKind')],
  ['GroupActionKind', schemaEnum('report.schema.json', 'GroupActionKind')],
  ['Freshness', schemaEnum('report.schema.json', 'Freshness')],
  ['NameKind', schemaEnum('metadata.schema.json', 'NameKind')],
  ['ReviewState', schemaEnum('metadata.schema.json', 'ReviewState')],
  ['Redistributable', schemaEnum('metadata.schema.json', 'Redistributable')],
  ['Source', schemaEnum('metadata.schema.json', 'Source')],
  ['ImageRoot', schemaEnum('basemap.schema.json', 'ImageRoot')],
  ['BasemapSource', schemaEnum('basemap.schema.json', 'Source')],
  ['Basemap', schemaEnum('basemap.schema.json', 'Basemap')],
  ['Transform', schemaEnum('basemap.schema.json', 'Transform')],
  ['BasemapReview', schemaEnum('basemap.schema.json', 'Review')],
  // catalog 的枚举是内联的，单独列出。
  ['CatalogAcquisition', new Set(['curated', 'game_data'])],
  ['CatalogNameSource', new Set(['game_locale', 'curated', 'zone_name'])],
  ['CatalogKind', new Set(['story', 'nonstory'])],
]

const escapeRe = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const glossaryHas = (value) =>
  new RegExp(`(^|[^A-Za-z0-9_])${escapeRe(value)}([^A-Za-z0-9_]|$)`).test(glossaryText)

const missingEnums = []
for (const [group, values] of enumGroups) {
  for (const value of values) {
    if (!glossaryHas(value)) missingEnums.push(`${group}.${value}`)
  }
}

/* ------------------------------------------------------------------ */
/* 结果                                                                */
/* ------------------------------------------------------------------ */

let failed = false

if (missingCodes.length) {
  failed = true
  console.error(`errors.md 缺少 ${missingCodes.length} 个源码错误码：`)
  for (const code of missingCodes) console.error(`  - ${code}`)
}

if (unknownCodes.length) {
  failed = true
  console.error(`errors.md 列出 ${unknownCodes.length} 个源码中不存在的码：`)
  for (const code of unknownCodes) console.error(`  - ${code}`)
}

if (missingEnums.length) {
  failed = true
  console.error(`glossary.md 缺少 ${missingEnums.length} 个枚举取值：`)
  for (const value of missingEnums) console.error(`  - ${value}`)
}

if (failed) {
  console.error('\n文档一致性校验失败。请更新 docs/reference/errors.md 与 docs/reference/glossary.md。')
  process.exit(1)
}

console.log(
  `docs consistency passed: ${sourceCodes.size} codes, ${enumGroups.reduce(
    (sum, [, values]) => sum + values.size,
    0,
  )} enum values`,
)
