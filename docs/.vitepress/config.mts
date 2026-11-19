import { defineConfig } from 'vitepress'
import { tokenize } from './search'

const repo = 'https://github.com/perillaroc/sottr-explorer'

const referenceSidebar = [
  { text: '参考', items: [{ text: '总览', link: '/reference/' }] },
  {
    text: '接口与命令行',
    items: [
      { text: '命令行', link: '/reference/cli' },
      { text: '契约 1.0.0', link: '/contracts' },
    ],
  },
  {
    text: '状态与术语',
    items: [
      { text: '错误码与诊断', link: '/reference/errors' },
      { text: '术语表', link: '/reference/glossary' },
    ],
  },
]

export default defineConfig({
  lang: 'zh-CN',
  title: 'SOTTR Explorer',
  description: '《古墓丽影：暗影》本地只读存档分析工具——使用与开发文档',

  cleanUrls: true,
  lastUpdated: true,
  sitemap: {
    hostname: process.env.URL || 'https://sottr-explorer.netlify.app',
  },
  head: [['meta', { name: 'theme-color', content: '#0f766e' }]],

  markdown: {
    lineNumbers: true,
  },

  themeConfig: {
    nav: [
      { text: '使用指南', link: '/guide/', activeMatch: '^/guide/' },
      { text: '开发文档', link: '/development/', activeMatch: '^/development/' },
      { text: '参考', link: '/reference/', activeMatch: '^/reference/' },
      { text: '数据与来源', link: '/data/', activeMatch: '^/data/' },
      { text: '许可', link: '/legal/license', activeMatch: '^/legal/' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: '使用指南',
          items: [
            { text: '总览', link: '/guide/' },
            { text: '快速开始', link: '/guide/getting-started' },
            { text: '界面导览', link: '/guide/interface' },
            { text: '位置与地图', link: '/guide/maps' },
            { text: '导出与分享', link: '/guide/export' },
            { text: '常见问题', link: '/guide/faq' },
          ],
        },
      ],

      '/development/': [
        {
          text: '开发文档',
          items: [{ text: '总览', link: '/development/' }],
        },
        {
          text: '系统',
          items: [
            { text: '架构总览', link: '/development/architecture' },
            { text: '数据管线', link: '/development/data-pipeline' },
            { text: '界面语言与文案', link: '/development/i18n' },
            { text: '安全模型', link: '/development/security' },
            { text: '已知限制', link: '/development/limitations' },
          ],
        },
        {
          text: '工程',
          items: [
            { text: '测试', link: '/development/testing' },
            { text: '构建', link: '/development/build' },
            { text: '发布', link: '/development/release' },
          ],
        },
        {
          text: '决策记录',
          items: [
            { text: '索引', link: '/development/decisions/' },
            { text: '0001 平台范围', link: '/development/decisions/0001-platform-scope' },
            { text: '0002 只读', link: '/development/decisions/0002-read-only' },
            { text: '0003 未知表达', link: '/development/decisions/0003-unknown-as-null' },
            { text: '0004 证据分层', link: '/development/decisions/0004-evidence-layers' },
            { text: '0005 不做概率', link: '/development/decisions/0005-no-probabilities' },
            { text: '0006 参考集合', link: '/development/decisions/0006-reference-set-exactly-one' },
            { text: '0007 坐标空间', link: '/development/decisions/0007-unit-local-coordinates' },
            { text: '0008 地图分发', link: '/development/decisions/0008-bundled-basemaps' },
            { text: '0009 双数据集', link: '/development/decisions/0009-dual-datasets' },
            { text: '0010 契约形态', link: '/development/decisions/0010-strict-contract' },
            { text: '0011 Python 参照', link: '/development/decisions/0011-python-oracle' },
          ],
        },
      ],

      '/reference/': referenceSidebar,
      '/contracts': referenceSidebar,

      '/data/': [
        {
          text: '数据与来源',
          items: [
            { text: '总览', link: '/data/' },
            { text: '全收集图鉴', link: '/data/catalog' },
            { text: '收集品元数据', link: '/data/metadata' },
            { text: '真实地图基底', link: '/data/map' },
            { text: '来源与许可', link: '/data/sources' },
          ],
        },
        {
          text: '研究记录',
          items: [
            { text: '总览', link: '/data/research/' },
            { text: '研究基线', link: '/data/research/baseline' },
            { text: '提取与容器格式', link: '/data/research/extraction' },
            { text: '研究发现', link: '/data/research/findings' },
            { text: '全收集对账', link: '/data/research/reconciliation' },
            { text: '整体完成度百分比', link: '/data/research/completion' },
            { text: 'DTP 64 位文本键', link: '/data/research/text-keys' },
            { text: 'missions 语义', link: '/data/research/missions' },
            { text: '用 mesh 生成缺失底图', link: '/data/research/mesh-basemaps' },
            { text: '存档布局扩展', link: '/data/research/save-layouts' },
            { text: '派生表口径', link: '/data/research/derived-tables' },
          ],
        },
      ],

      '/legal/': [
        {
          text: '许可与声明',
          items: [
            { text: '产品许可', link: '/legal/license' },
            { text: '第三方声明', link: '/legal/third-party' },
          ],
        },
      ],
    },

    search: {
      provider: 'local',
      options: {
        translations: {
          button: { buttonText: '搜索文档' },
          modal: {
            displayDetails: '展开详细列表',
            resetButtonTitle: '清除搜索',
            backButtonTitle: '关闭搜索',
            noResultsText: '没有找到相关结果',
            footer: {
              selectText: '选择',
              selectKeyAriaLabel: '回车',
              navigateText: '切换',
              navigateUpKeyAriaLabel: '上箭头',
              navigateDownKeyAriaLabel: '下箭头',
              closeText: '关闭',
              closeKeyAriaLabel: 'Esc',
            },
          },
        },
        miniSearch: {
          options: {
            tokenize,
          },
        },
      },
    },

    socialLinks: [{ icon: 'github', link: repo }],

    outline: { level: [2, 3], label: '本页目录' },
    skipToContentLabel: '跳到正文',
    lastUpdated: {
      text: '最后更新',
      formatOptions: { dateStyle: 'short', timeStyle: 'short' },
    },
    docFooter: { prev: '上一篇', next: '下一篇' },
    editLink: {
      pattern: `${repo}/edit/main/docs/:path`,
      text: '在 GitHub 上编辑此页',
    },
    darkModeSwitchLabel: '外观',
    lightModeSwitchTitle: '切换到浅色模式',
    darkModeSwitchTitle: '切换到深色模式',
    sidebarMenuLabel: '菜单',
    returnToTopLabel: '回到顶部',
    langMenuLabel: '语言',

    footer: {
      message: '代码采用 MIT；游戏数据事实与地图素材各自保留原权利。',
      copyright:
        '非官方、只读工具，与 Square Enix / Crystal Dynamics 无从属关系。',
    },
  },
})
