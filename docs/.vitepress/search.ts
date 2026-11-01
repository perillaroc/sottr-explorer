/**
 * 中文分词：VitePress 本地搜索基于 MiniSearch，默认分词对连续中文不友好。
 * 构建期用它生成索引，客户端主题在 setup 时把同一个函数注入查询选项，
 * 保证索引与查询使用同一套切词规则。
 */
const segmenter = new Intl.Segmenter('zh-CN', { granularity: 'word' })

export function tokenize(text: string): string[] {
  return Array.from(segmenter.segment(text), (item) => item.segment).filter(
    (item) => item.trim().length > 0,
  )
}
