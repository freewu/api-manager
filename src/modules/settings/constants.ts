import { ExportFormat, ImportFormat } from "../../types";

/** 外观：主题模式选项 */
export const MODES = [
  { value: "dark", labelKey: "settings.mode.dark" },
  { value: "light", labelKey: "settings.mode.light" },
  { value: "system", labelKey: "settings.mode.system" },
] as const;

/** 关于页：项目地址 / 反馈地址 */
export const PROJECT_URL = "https://github.com/freewu/api-manager";
export const ISSUE_URL = "https://github.com/freewu/api-manager/issues/new";

/** 导入格式列表（含可开关的） */
export const IMPORT_FORMATS: { value: ImportFormat; labelKey: string }[] = [
  { value: "postman", labelKey: "export.postman" },
  { value: "openapi", labelKey: "export.openapi" },
  { value: "markdown", labelKey: "export.markdown" },
  { value: "apifox", labelKey: "export.apifox" },
  { value: "apipost", labelKey: "export.apipost" },
  { value: "raml", labelKey: "export.raml" },
  { value: "wadl", labelKey: "export.wadl" },
  { value: "har", labelKey: "export.har" },
  { value: "yapi", labelKey: "export.yapi" },
  { value: "eolink", labelKey: "export.eolink" },
  { value: "insomnia", labelKey: "export.insomnia" },
  { value: "jmeter", labelKey: "export.jmeter" },
  { value: "apidoc", labelKey: "export.apidoc" },
  { value: "apidog", labelKey: "export.apidog" },
  { value: "bruno", labelKey: "export.bruno" },
  { value: "apizza", labelKey: "export.apizza" },
  { value: "nei", labelKey: "export.nei" },
  { value: "doclever", labelKey: "export.doclever" },
  { value: "io-docs", labelKey: "export.io-docs" },
  { value: "easydoc", labelKey: "export.easydoc" },
  { value: "docway", labelKey: "export.docway" },
  { value: "hoppscotch", labelKey: "export.hoppscotch" },
  { value: "metersphere", labelKey: "export.metersphere" },
  { value: "rap2", labelKey: "export.rap2" },
  { value: "curl", labelKey: "export.curl" },
];

/** 导出格式列表（含可开关的） */
export const EXPORT_FORMATS: { value: ExportFormat; labelKey: string }[] = [
  { value: "postman", labelKey: "export.postman" },
  { value: "openapi", labelKey: "export.openapi" },
  { value: "docsify", labelKey: "export.docsify" },
  { value: "mkdocs", labelKey: "export.mkdocs" },
  { value: "markdown", labelKey: "export.markdown" },
  { value: "html", labelKey: "export.html" },
  { value: "apifox", labelKey: "export.apifox" },
  { value: "apipost", labelKey: "export.apipost" },
  { value: "raml", labelKey: "export.raml" },
  { value: "wadl", labelKey: "export.wadl" },
  { value: "yapi", labelKey: "export.yapi" },
  { value: "eolink", labelKey: "export.eolink" },
  { value: "insomnia", labelKey: "export.insomnia" },
  { value: "jmeter", labelKey: "export.jmeter" },
  { value: "apidoc", labelKey: "export.apidoc" },
  { value: "apidog", labelKey: "export.apidog" },
  { value: "bruno", labelKey: "export.bruno" },
  { value: "apizza", labelKey: "export.apizza" },
  { value: "nei", labelKey: "export.nei" },
  { value: "doclever", labelKey: "export.doclever" },
  { value: "io-docs", labelKey: "export.io-docs" },
  { value: "easydoc", labelKey: "export.easydoc" },
  { value: "docway", labelKey: "export.docway" },
  { value: "hoppscotch", labelKey: "export.hoppscotch" },
  { value: "metersphere", labelKey: "export.metersphere" },
  { value: "rap2-project", labelKey: "export.rap2-project" },
];

/** 左侧导航（目录）项：点击滚动到对应分区 */
export const NAV = [
  { id: "workspace", icon: "📁", titleKey: "settings.nav.workspace", descKey: "settings.nav.workspaceDesc" },
  { id: "language", icon: "🌐", titleKey: "settings.nav.language", descKey: "settings.nav.languageDesc" },
  { id: "appearance", icon: "🎨", titleKey: "settings.nav.appearance", descKey: "settings.nav.appearanceDesc" },
  { id: "version", icon: "📦", titleKey: "settings.nav.version", descKey: "settings.nav.versionDesc" },
  { id: "mock", icon: "🛡️", titleKey: "settings.nav.mock", descKey: "settings.nav.mockDesc" },
  { id: "codegen", icon: "💻", titleKey: "settings.nav.codegen", descKey: "settings.nav.codegenDesc" },
  { id: "export", icon: "📤", titleKey: "settings.nav.export", descKey: "settings.nav.exportDesc" },
  { id: "import", icon: "📥", titleKey: "settings.nav.import", descKey: "settings.nav.importDesc" },
  { id: "headers", icon: "🧾", titleKey: "settings.nav.headers", descKey: "settings.nav.headersDesc" },
  { id: "sync", icon: "🔄", titleKey: "settings.nav.sync", descKey: "settings.nav.syncDesc" },
  { id: "about", icon: "ℹ️", titleKey: "settings.nav.about", descKey: "settings.nav.aboutDesc" },
] as const;
