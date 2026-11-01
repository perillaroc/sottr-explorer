import DefaultTheme from 'vitepress/theme'
import { useData } from 'vitepress'
import { tokenize } from '../search'
import './custom.css'

export default {
  extends: DefaultTheme,
  setup() {
    const { theme } = useData()
    const search = theme.value.search

    if (search?.provider === 'local') {
      // themeConfig 由构建期序列化成 JSON，函数不会被带过去；
      // 在客户端 setup 时注入，与构建期索引使用同一个分词器。
      const options = (search.options ??= {}) as Record<string, any>
      options.miniSearch ??= {}
      options.miniSearch.options ??= {}
      options.miniSearch.options.tokenize = tokenize
    }
  },
}
