import { defineConfig } from 'vitepress'

const repository = 'https://github.com/bahayonghang/devsweep'
const editPattern = `${repository}/edit/main/docs/:path`

const englishNav = [
  { text: 'Guide', link: '/guide/getting-started' },
  { text: 'Reference', link: '/reference/cli' },
  { text: 'CI', link: '/ci' },
  { text: 'GitHub', link: repository },
]

const chineseNav = [
  { text: '指南', link: '/zh/guide/getting-started' },
  { text: '参考', link: '/zh/reference/cli' },
  { text: 'CI', link: '/zh/ci' },
  { text: 'GitHub', link: repository },
]

const englishSidebar = {
  '/guide/': [
    {
      text: 'Guide',
      items: [
        { text: 'Getting Started', link: '/guide/getting-started' },
        { text: 'Safety Model', link: '/guide/safety-model' },
        { text: 'Terminal UI', link: '/guide/tui' },
        { text: 'Scan', link: '/guide/scan' },
        { text: 'Inventory', link: '/guide/inventory' },
        { text: 'Clean a Saved Plan', link: '/guide/clean' },
        { text: 'Protect Paths', link: '/guide/protection' },
      ],
    },
  ],
  '/reference/': [
    {
      text: 'Reference',
      items: [
        { text: 'CLI', link: '/reference/cli' },
        { text: 'Plans and Reports', link: '/reference/plan-and-report' },
        { text: 'Rules', link: '/reference/rules' },
        { text: 'CI and Releases', link: '/ci' },
        { text: 'Provenance', link: '/provenance' },
      ],
    },
  ],
}

const chineseSidebar = {
  '/zh/guide/': [
    {
      text: '指南',
      items: [
        { text: '快速开始', link: '/zh/guide/getting-started' },
        { text: '安全模型', link: '/zh/guide/safety-model' },
        { text: '终端界面', link: '/zh/guide/tui' },
        { text: '扫描', link: '/zh/guide/scan' },
        { text: '盘点', link: '/zh/guide/inventory' },
        { text: '清理已保存计划', link: '/zh/guide/clean' },
        { text: '保护路径', link: '/zh/guide/protection' },
      ],
    },
  ],
  '/zh/reference/': [
    {
      text: '参考',
      items: [
        { text: '命令行', link: '/zh/reference/cli' },
        { text: '计划与报告', link: '/zh/reference/plan-and-report' },
        { text: '规则', link: '/zh/reference/rules' },
        { text: 'CI 与发布', link: '/zh/ci' },
        { text: '溯源', link: '/zh/provenance' },
      ],
    },
  ],
}

const englishTheme = {
  nav: englishNav,
  sidebar: englishSidebar,
  editLink: {
    pattern: editPattern,
    text: 'Edit this page on GitHub',
  },
  outline: {
    label: 'On this page',
  },
  docFooter: {
    prev: 'Previous page',
    next: 'Next page',
  },
  footer: {
    message: 'Released under the MIT License.',
    copyright: 'Copyright 2026 DevSweep contributors',
  },
}

const chineseTheme = {
  nav: chineseNav,
  sidebar: chineseSidebar,
  editLink: {
    pattern: editPattern,
    text: '在 GitHub 上编辑此页',
  },
  outline: {
    label: '本页内容',
  },
  docFooter: {
    prev: '上一页',
    next: '下一页',
  },
  footer: {
    message: '基于 MIT 许可证发布。',
    copyright: 'Copyright 2026 DevSweep contributors',
  },
}

export default defineConfig({
  lang: 'en-US',
  title: 'DevSweep',
  description: 'Safety-first developer cleanup planning and execution.',
  lastUpdated: true,
  srcExclude: ['agents/**'],
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      themeConfig: englishTheme,
    },
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      title: 'DevSweep',
      description: '安全优先的开发环境清理规划与执行工具。',
      themeConfig: chineseTheme,
    },
  },
  themeConfig: englishTheme,
})
