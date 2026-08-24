import { useState } from "react";
import {
  copyEntry,
  createApi,
  createFolder,
  deleteEntry,
  exportApiMarkdown,
  exportSelection,
  listVersions,
  readApi,
  readInfo,
  renameEntry,
  renderApiMarkdown,
  renderGroupMarkdown,
  saveApi,
  saveInfo,
  type ExportFormat,
  type MarkdownDoc,
} from "../commands";
import { buildApiDocComment } from "../utils/apidoc";
import { ApiFile, AppSettings, TreeNode, VersionInfo } from "../types";
import { InfoForm, ModalState, emptyInfoForm } from "../components/AppModals";
import { parseCurl } from "../utils/curl";

/**
 * 弹窗操作：新建接口/分组/重命名/删除/分组信息、版本管理、
 * Markdown / apiDoc 预览、导出弹窗。
 */
export function useModals(opts: {
  workspace: string | null;
  selectedPath: string | null;
  settings: AppSettings;
  reloadTree: (showLoading?: boolean) => Promise<void>;
  reloadMockIfRunning: (running: boolean) => Promise<void>;
  onApiReplaced: (data: ApiFile, path: string) => void; // 读接口后更新编辑区
  onApiCleared: () => void; // 当前编辑接口被删除时清空编辑区
  onToast: (msg: string) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}) {
  const {
    workspace,
    selectedPath,
    settings,
    reloadTree,
    reloadMockIfRunning,
    onApiReplaced,
    onApiCleared,
    onToast,
    t,
  } = opts;

  const [modal, setModal] = useState<ModalState | null>(null);
  const [modalText, setModalText] = useState("");
  const [modalProtocol, setModalProtocol] = useState<"http" | "websocket" | "graphql" | "socketio">("http");
  const [infoForm, setInfoForm] = useState<InfoForm>(emptyInfoForm());
  const [demoCreate, setDemoCreate] = useState(true);
  const [versionModal, setVersionModal] = useState<{ api: ApiFile; versions: VersionInfo[] } | null>(null);
  const [statsNode, setStatsNode] = useState<TreeNode | null>(null);
  const [mdView, setMdView] = useState<{ node: TreeNode; doc: MarkdownDoc } | null>(null);
  const [apiDocView, setApiDocView] = useState<{ node: TreeNode; text: string } | null>(null);
  const [exportOpen, setExportOpen] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportPreselect, setExportPreselect] = useState<string[] | undefined>(undefined);
  const [curlOpen, setCurlOpen] = useState(false);
  const [curlName, setCurlName] = useState("");
  const [curlText, setCurlText] = useState("");
  const [curlError, setCurlError] = useState("");
  const [notify, setNotify] = useState<{ title: string; body: string } | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const openModal = (type: ModalState["type"], parent = "", target?: TreeNode) => {
    setModalText(target?.name || (type === "newApi" ? t("app.unnamedApi") : type === "newFolder" ? t("app.newFolder") : ""));
    setModalProtocol("http");
    setModal({ type, parent, target });
  };

  const openInfoModal = async (target: TreeNode) => {
    try {
      const info = await readInfo(target.path);
      setInfoForm({
        name: info.name || target.name,
        description: info.description || "",
      });
      setModal({ type: "info", parent: target.path, target });
    } catch (e) {
      onToast(t("toast.readInfoFailed", { err: String(e) }));
    }
  };

  const doNewApi = async () => {
    if (!modal) return;
    const name = modalText.trim() || t("app.unnamedApi");
    try {
      const dir = modal.parent || workspace!;
      const path = await createApi(dir, name, modalProtocol);
      setModal(null);
      await reloadTree();
      const data = await readApi(path);
      // 设置开启「默认 Header」时，新接口自动附带默认请求头并落盘
      if (settings.enableDefaultHeaders) {
        const defaults = (settings.defaultHeaders || []).filter((h) => h.key.trim());
        if (defaults.length > 0) {
          data.headers = [...defaults.map((h) => ({ ...h })), ...data.headers];
          await saveApi(path, data);
        }
      }
      onApiReplaced(data, path);
      void reloadMockIfRunning(true);
      onToast(t("toast.createdApi", { name }));
    } catch (e) {
      onToast(t("toast.failed", { err: String(e) }));
    }
  };

  /** 打开「从 Curl 导入」弹窗 */
  const openCurlImport = () => {
    setCurlName("");
    setCurlText("");
    setCurlError("");
    setCurlOpen(true);
  };

  /** 解析 curl 并创建 http 接口 */
  const doImportCurl = async () => {
    const name = curlName.trim() || t("app.unnamedApi");
    let parsed;
    try {
      parsed = parseCurl(curlText);
    } catch (e) {
      setCurlError(t("modal.curlParseError", { err: String(e) }));
      return;
    }
    try {
      const path = await createApi(workspace!, name);
      const data = await readApi(path);
      data.method = parsed.method;
      data.url = parsed.url;
      data.query = parsed.query;
      data.headers = parsed.headers;
      if (parsed.bodyMode !== "none") {
        data.body = {
          ...data.body,
          mode: parsed.bodyMode === "json" ? "json" : parsed.bodyMode === "form" ? "form" : "raw",
          raw: parsed.bodyRaw,
        };
      }
      await saveApi(path, data);
      setCurlOpen(false);
      await reloadTree();
      onApiReplaced(data, path);
      void reloadMockIfRunning(true);
      onToast(t("toast.importedCurl", { name }));
    } catch (e) {
      onToast(t("toast.failed", { err: String(e) }));
    }
  };

  const doNewFolder = async () => {
    if (!modal) return;
    const name = modalText.trim() || t("app.newFolder");
    try {
      const parent = modal.parent || workspace!;
      await createFolder(parent, name);
      setModal(null);
      await reloadTree();
      onToast(t("toast.createdFolder", { name }));
    } catch (e) {
      onToast(t("toast.failed", { err: String(e) }));
    }
  };

  const doRename = async () => {
    if (!modal?.target) return;
    const name = modalText.trim();
    if (!name) return;
    try {
      await renameEntry(modal.target.path, name);
      setModal(null);
      await reloadTree();
      onToast(t("toast.renamed"));
    } catch (e) {
      onToast(t("toast.renameFailed", { err: String(e) }));
    }
  };

  const handleCopy = async (node: TreeNode) => {
    try {
      const p = await copyEntry(node.path);
      await reloadTree();
      void reloadMockIfRunning(true);
      onToast(t("toast.copied", { name: p }));
    } catch (e) {
      onToast(t("toast.failed", { err: String(e) }));
    }
  };

  const doDelete = async () => {
    if (!modal?.target) return;
    try {
      await deleteEntry(modal.target.path);
      setModal(null);
      if (selectedPath === modal.target.path) {
        onApiCleared();
      }
      await reloadTree();
      onToast(t("toast.deleted"));
    } catch (e) {
      onToast(t("toast.deleteFailed", { err: String(e) }));
    }
  };

  const doSaveInfo = async () => {
    if (!modal) return;
    try {
      await saveInfo(modal.parent, {
        name: infoForm.name.trim() || undefined,
        description: infoForm.description,
      });
      setModal(null);
      await reloadTree();
      onToast(t("toast.saved"));
    } catch (e) {
      onToast(t("toast.saveFailed", { err: String(e) }));
    }
  };

  // 查看接口版本信息（右键 -> 查看版本信息）
  const openVersions = async (node: TreeNode) => {
    try {
      const data = await readApi(node.path);
      if (!data.uuid) data.uuid = crypto.randomUUID();
      const versions = await listVersions(data.uuid);
      setVersionModal({ api: data, versions });
    } catch (e) {
      onToast(t("toast.readVersionsFailed", { err: String(e) }));
    }
  };

  /** 版本恢复成功后：重新加载接口内容并刷新左侧树/版本号 */
  const handleVersionRestored = async (path: string, version: number) => {
    try {
      const data = await readApi(path);
      if (!data.uuid) data.uuid = crypto.randomUUID();
      onApiReplaced(data, path);
      void reloadTree();
      setVersionModal(null);
      onToast(t("version.restored", { version }));
    } catch (e) {
      onToast(t("toast.saveFailed", { err: String(e) }));
    }
  };

  /** 查看接口 / 分组的 Markdown 格式（预览弹窗，可保存 .md / .html） */
  const handleViewMarkdown = async (node: TreeNode) => {
    try {
      const doc =
        node.kind === "folder"
          ? await renderGroupMarkdown(node.path)
          : await renderApiMarkdown(node.path);
      setMdView({ node, doc });
    } catch (e) {
      onToast(t("toast.markdownFailed", { err: String(e) }));
    }
  };

  /** 查看接口 apiDoc 注释（可一键复制） */
  const handleViewApiDoc = async (node: TreeNode) => {
    try {
      const api = await readApi(node.path);
      const groupPath = node.path.split(/[\\/]/).slice(0, -1).join("/");
      setApiDocView({ node, text: buildApiDocComment(api, groupPath) });
    } catch (e) {
      onToast(t("toast.markdownFailed", { err: String(e) }));
    }
  };

  /** 保存 Markdown / HTML 文件到用户选择的目录 */
  const handleExportMarkdown = async (format: "md" | "html") => {
    if (!mdView) return;
    setExporting(true);
    try {
      const saved = await exportApiMarkdown(mdView.node.path, format, settings.htmlNav);
      if (saved) onToast(t("toast.savedTo", { path: saved }));
    } catch (e) {
      onToast(t("toast.saveFailed", { err: String(e) }));
    } finally {
      setExporting(false);
    }
  };

  /** 导出选中接口/分组为 Postman / OpenAPI / Docsify / Markdown / HTML 等格式 */
  const handleExport = async (paths: string[], format: ExportFormat) => {
    setExporting(true);
    try {
      const saved = await exportSelection(paths, format, settings.htmlNav);
      if (!saved) return; // 用户取消
      const kind = format === "docsify" ? t("export.kindDir") : t("export.kindFile");
      onToast(t("toast.exported", { kind, path: saved }));
      setExportOpen(false);
      setExportPreselect(undefined);
    } catch (e) {
      onToast(t("toast.exportFailed", { err: String(e) }));
    } finally {
      setExporting(false);
    }
  };

  /** 打开导出弹窗（可选预选某个节点） */
  const openExport = (node?: TreeNode) => {
    setExportPreselect(node ? [node.path] : undefined);
    setExportOpen(true);
  };

  return {
    modal,
    setModal,
    modalText,
    setModalText,
    modalProtocol,
    setModalProtocol,
    infoForm,
    setInfoForm,
    demoCreate,
    setDemoCreate,
    versionModal,
    setVersionModal,
    statsNode,
    setStatsNode,
    mdView,
    setMdView,
    apiDocView,
    setApiDocView,
    exportOpen,
    setExportOpen,
    exporting,
    exportPreselect,
    setExportPreselect,
    notify,
    setNotify,
    settingsOpen,
    setSettingsOpen,
    openModal,
    openInfoModal,
    doNewApi,
    doNewFolder,
    doRename,
    handleCopy,
    doDelete,
    doSaveInfo,
    openVersions,
    handleVersionRestored,
    handleViewMarkdown,
    handleViewApiDoc,
    handleExportMarkdown,
    handleExport,
    openExport,
    curlOpen,
    setCurlOpen,
    curlName,
    setCurlName,
    curlText,
    setCurlText,
    curlError,
    setCurlError,
    openCurlImport,
    doImportCurl,
  };
}
