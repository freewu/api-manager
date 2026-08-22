import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createApi,
  createDemo,
  createFolder,
  copyEntry,
  deleteEntry,
  getAppVersion,
  getWorkspace,
  getRecentWorkspaces,
  openWorkspace,
  importOpenApi,
  importPostman,
  importApifox,
  importApipost,
  importRaml,
  importWadl,
  importHar,
  importYapi,
  importEolink,
  importInsomnia,
  importMarkdown,
  renderApiMarkdown,
  renderGroupMarkdown,
  exportApiMarkdown,
  exportSelection,
  type MarkdownDoc,
  type ExportFormat,
  listVersions,
  loadSettings,
  mockReload,
  mockStart,
  mockStatus,
  mockStop,
  pickWorkspace,
  readApi,
  readApiVersion,
  readEnv,
  readInfo,
  readTree,
  renameEntry,
  moveEntry,
  saveApi,
  saveApiVersion,
  saveEnv,
  saveInfo,
  saveSettings,
  sendRequest,
  saveHistory,
  saveExample,
  toggleDeprecated,
  updateTrayEnv,
  vcsCommitPush,
  vcsInfo,
  vcsSync,
  hasWorkspaceInfo,
  getCurrentVersion,
} from "./commands";
import { AppModals, InfoForm, ModalState, emptyInfoForm } from "./components/AppModals";
import { AppToolbar } from "./components/AppToolbar";
import { Landing } from "./components/Landing";
import { RightPane } from "./components/RightPane";
import { AppView, Sidebar } from "./components/Sidebar";
import { useHistory } from "./hooks/useHistory";
import { setLang, useT } from "./i18n";

/** 转义正则特殊字符（用于按字面量构造 {变量名} 匹配） */
const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** 归一化语言值：兼容旧配置 "" / "zh" / "en" 与新值 "zh-tw" */
const normalizeLang = (v: unknown): "zh" | "zh-tw" | "en" => {
  const s = String(v || "").toLowerCase().replace(/_/g, "-");
  if (s === "en") return "en";
  if (s === "zh-tw" || s === "zh-hant" || s === "zh-cht" || s === "tw" || s === "cht") return "zh-tw";
  return "zh";
};

import {
  ApiFile,
  AppSettings,
  BodyData,
  EnvStore,
  EnvVariable,
  HttpRequestData,
  HttpResult,
  InfoJson,
  MockStatus,
  TreeNode,
  UpdateInfo,
  VersionInfo,
  WsLogEntry,
  defaultSettings,
  emptyEnv,
} from "./types";

export default function App() {
  const t = useT();
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [recent, setRecent] = useState<string[]>([]);
  const [tree, setTree] = useState<TreeNode | null>(null);
  // 工作目录树加载中（首次加载 / 打开工作区 / 手动刷新时显示加载动画）
  const [treeLoading, setTreeLoading] = useState(false);
  const [rootInfo, setRootInfo] = useState<InfoJson>({});
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [api, setApi] = useState<ApiFile | null>(null);
  const [dirty, setDirty] = useState(false);
  const [response, setResponse] = useState<HttpResult | null>(null);
  const [lastRequest, setLastRequest] = useState<HttpRequestData | null>(null);
  /** 发送请求时的接口快照（用于保存示例时记录 path/query 结构化参数） */
  const [lastApiSnapshot, setLastApiSnapshot] = useState<ApiFile | null>(null);
  // Mock / 描述 / 接口文档 / 生成代码 页签下隐藏响应面板
  const [hideResponse, setHideResponse] = useState(false);
  const [sending, setSending] = useState(false);
  const [mock, setMock] = useState<MockStatus>({ running: false, routeCount: 0 });
  const [vcs, setVcs] = useState<"git" | "svn" | null>(null);
  const [modal, setModal] = useState<ModalState | null>(null);
  const [modalText, setModalText] = useState("");
  const [modalProtocol, setModalProtocol] = useState<"http" | "websocket">("http");
  const [infoForm, setInfoForm] = useState<InfoForm>(emptyInfoForm());
  const [toast, setToast] = useState<string | null>(null);
  /** 发现新版本信息（托盘检查更新后通过事件推送，弹窗提醒） */
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  /** 更新提醒弹窗是否显示（关闭弹窗后 logo 上的「新版本」徽标仍在） */
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  /** 右下角持久弹窗（不自动消失），用于同步/提交等错误提示 */
  const [notify, setNotify] = useState<{ title: string; body: string } | null>(null);
  const [version, setVersion] = useState("");
  /** 当前接口已保存的最新版本号（.version/<uuid> 最大版本，未保存过为 0） */
  const [currentVersion, setCurrentVersion] = useState(0);
  /** 示例保存版本号：保存示例成功后 +1，驱动 Editor「示例」角标刷新计数 */
  const [exampleVersion, setExampleVersion] = useState(0);
  const [envs, setEnvs] = useState<EnvStore>(emptyEnv());
  const [envModal, setEnvModal] = useState(false);
  const [envValue, setEnvValue] = useState(false);
  const [versionModal, setVersionModal] = useState<{ api: ApiFile; versions: VersionInfo[] } | null>(null);
  const [statsNode, setStatsNode] = useState<TreeNode | null>(null);
  const [emptyMenu, setEmptyMenu] = useState<{ x: number; y: number } | null>(null);
  /** 接口 Markdown 文档预览弹窗 */
  const [mdView, setMdView] = useState<{ node: TreeNode; doc: MarkdownDoc } | null>(null);
  /** 导出弹窗：preselect 为右键节点预选路径 */
  const [exportOpen, setExportOpen] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportPreselect, setExportPreselect] = useState<string[] | undefined>(undefined);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings());
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [view, setView] = useState<AppView>("api");
  const history = useHistory();
  const switchView = (v: AppView) => {
    setView(v);
    // 每次进入历史视图都自动刷新一次列表
    if (v === "history") history.reload();
  };
  const [demoCreate, setDemoCreate] = useState(true);
  const [sidebarWidth, setSidebarWidth] = useState(() => {
    const saved = Number(localStorage.getItem("sidebar-width"));
    return saved >= 200 && saved <= 640 ? saved : 310;
  });
  const sidebarWidthRef = useRef(sidebarWidth);
  // 编辑器 / 响应面板的高度比例（可拖动分栏调整）
  const [editorRatio, setEditorRatio] = useState(() => {
    const saved = Number(localStorage.getItem("editor-ratio"));
    return saved >= 0.2 && saved <= 0.8 ? saved : 0.45;
  });
  const editorRatioRef = useRef(editorRatio);
  const toastTimer = useRef<number | null>(null);

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2200);
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const v = await getAppVersion();
        setVersion(v);
      } catch {
        /* noop */
      }
      const ws = await getWorkspace();
      if (ws) {
        setWorkspace(ws);
        if (!(await hasWorkspaceInfo())) {
          // 新的工作目录（没有 __info.json）：询问是否生成演示案例
          setDemoCreate(true);
          setModal({ type: "demo", parent: ws });
        } else {
          await loadAll(ws);
        }
      }
    })();
    // 加载最近打开的工作目录（开始页展示）
    getRecentWorkspaces()
      .then(setRecent)
      .catch(() => {});

    loadSettings()
      .then((s) => {
        // 旧配置可能缺少 importTypes/exportTypes 开关，合并默认值保证字段存在
        const def = defaultSettings();
        setSettings({
          ...def,
          ...s,
          importTypes: { ...def.importTypes, ...(s.importTypes || {}) },
          exportTypes: { ...def.exportTypes, ...(s.exportTypes || {}) },
        });
        setLang(normalizeLang(s.language));
      })
      .catch(() => {});
    mockStatus().then(setMock).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 托盘菜单切换语言后，前端同步刷新文案与设置状态
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("language-changed", (e) => {
        const l = normalizeLang(e.payload);
        setLang(l);
        setSettings((s) => ({ ...s, language: l }));
      });
    })();
    return () => unlisten?.();
  }, []);

  // 托盘检查更新发现新版本后，弹出更新提醒弹窗
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("update-available", (e) => {
        setUpdateInfo(e.payload as UpdateInfo);
        setShowUpdateModal(true);
      });
    })();
    return () => unlisten?.();
  }, []);

  // 窗口标题带上版本号
  useEffect(() => {
    if (!version) return;
    const title = `API Manager v${version}`;
    document.title = title;
    getCurrentWindow().setTitle(title).catch(() => {});
  }, [version]);

  // 应用显示模式（深色 / 浅色 / 跟随系统）
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const mode =
        settings.displayMode === "system"
          ? mq.matches
            ? "dark"
            : "light"
          : settings.displayMode;
      document.documentElement.setAttribute("data-theme", mode);
    };
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings.displayMode]);

  // 托盘菜单点击「环境变量」-> 打开环境变量编辑器
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("open-env-editor", () => setEnvModal(true));
    })();
    return () => unlisten?.();
  }, []);

  // 托盘 Mock 服务启动/停止后，主页面联动刷新状态并提示
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("mock-status-changed", async () => {
        try {
          const s = await mockStatus();
          setMock(s);
          showToast(s.running ? t("mock.starting", { port: s.port || settings.mockPort }) : t("mock.stopped"));
        } catch {
          /* noop */
        }
      });
    })();
    return () => unlisten?.();
  }, []);

  // 空区域右键菜单：点击任意处 / Esc 关闭
  useEffect(() => {
    if (!emptyMenu) return;
    const close = () => setEmptyMenu(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("click", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [emptyMenu]);

  /** 查询接口当前版本号（保存按钮 tip 展示用） */
  const refreshVersion = useCallback(async (uuid: string) => {
    try {
      setCurrentVersion(await getCurrentVersion(uuid));
    } catch {
      setCurrentVersion(0);
    }
  }, []);

  async function loadAll(ws: string) {
    setTreeLoading(true);
    try {
      const [t, info, e] = await Promise.all([readTree(), readInfo(ws), readEnv()]);
      setTree(t);
      setRootInfo(info || {});
      const envData = e || emptyEnv();
      setEnvs(envData);
      updateTrayEnv(envData.active || "").catch(() => {});
      // 检测工作目录版本控制（.git / .svn）
      vcsInfo().then((r) => setVcs(r.vcs)).catch(() => setVcs(null));
      // 自动选中第一个接口
      const first = findFirstApi(t);
      if (first) {
        setSelectedPath(first.path);
        const data = await readApi(first.path);
        setApi(data);
        setDirty(false);
        void refreshVersion(data.uuid);
      } else {
        setSelectedPath(null);
        setApi(null);
        setCurrentVersion(0);
      }
    } finally {
      setTreeLoading(false);
    }
  }

  function findFirstApi(node: TreeNode): TreeNode | null {
    if (node.kind === "api") return node;
    for (const c of node.children || []) {
      const r = findFirstApi(c);
      if (r) return r;
    }
    return null;
  }

  async function reloadTree(showLoading = false) {
    if (showLoading) setTreeLoading(true);
    try {
      const t = await readTree();
      setTree(t);
    } finally {
      if (showLoading) setTreeLoading(false);
    }
  }

  // 递归更新树中指定路径节点的 method（修改 HTTP 方法时即时刷新左侧徽标）
  function patchNodeMethod(node: TreeNode, path: string, method: string): TreeNode {
    if (node.path === path) return { ...node, method };
    if (node.children) {
      return { ...node, children: node.children.map((c) => patchNodeMethod(c, path, method)) };
    }
    return node;
  }

  // 递归更新树中指定路径节点的 mock 状态（切换 Mock 开关时即时刷新左侧圆点）
  function patchNodeMock(node: TreeNode, path: string, mockEnabled: boolean): TreeNode {
    if (node.path === path) return { ...node, mockEnabled };
    if (node.children) {
      return { ...node, children: node.children.map((c) => patchNodeMock(c, path, mockEnabled)) };
    }
    return node;
  }

  useEffect(() => {
    if (!api || !selectedPath) return;
    setTree((t) => (t ? patchNodeMethod(t, selectedPath, api.method) : t));
  }, [selectedPath, api?.method]);

  // Mock 开关变化时同步刷新左侧列表的 Mock 圆点
  useEffect(() => {
    if (!api || !selectedPath) return;
    setTree((t) => (t ? patchNodeMock(t, selectedPath, !!api.mock?.enabled) : t));
  }, [selectedPath, api?.mock?.enabled]);

  const selectNode = useCallback(
    async (node: TreeNode) => {
      if (dirty && api) {
        try {
          await saveApi(selectedPath!, api);
          showToast(t("toast.autoSaved"));
        } catch (e) {
          console.error(e);
        }
      }
      setSelectedPath(node.path);
      const data = await readApi(node.path);
      // 旧接口文件无 uuid 时自动补一个，保证版本目录与接口一一对应
      if (!data.uuid) data.uuid = crypto.randomUUID();
      setApi(data);
      setDirty(false);
      setResponse(null);
      void refreshVersion(data.uuid);
    },
    [dirty, api, selectedPath, showToast]
  );

  /** 打开工作目录后的统一收尾：加载树/信息并处理新目录询问 */
  const finishOpenWorkspace = async (ws: string) => {
    setWorkspace(ws);
    setResponse(null);
    if (!(await hasWorkspaceInfo())) {
      // 新的工作目录（没有 __info.json）：询问是否生成演示案例
      setDemoCreate(true);
      setModal({ type: "demo", parent: ws });
    } else {
      await loadAll(ws);
      showToast(t("toast.opened"));
    }
  };

  /** 最近打开的工作目录展示数量（设置项，仅影响显示，历史记录不删除） */
  const recentLimit = Math.max(3, settings.recentLimit || 5);

  /** 把目录加入最近打开列表（本地即时更新，后端已持久化；保留全部记录） */
  const pushRecent = (ws: string) => {
    setRecent((r) => [ws, ...r.filter((x) => x !== ws)]);
  };

  const handlePickWorkspace = async () => {
    try {
      const ws = await pickWorkspace();
      if (!ws) return;
      pushRecent(ws);
      await finishOpenWorkspace(ws);
    } catch (e) {
      showToast(t("toast.openFailed", { err: String(e) }));
    }
  };

  /** 开始页「最近打开」：按路径直接打开 */
  const handleOpenRecent = async (ws: string) => {
    try {
      await openWorkspace(ws);
      pushRecent(ws);
      await finishOpenWorkspace(ws);
    } catch (e) {
      showToast(t("toast.openFailed", { err: String(e) }));
    }
  };

  /** 新工作目录（无 __info.json）询问后的收尾：按参数生成演示案例并加载 */
  const closeDemoModal = async (create: boolean) => {
    const ws = modal?.parent || workspace;
    setModal(null);
    if (ws) {
      if (create) {
        try {
          await createDemo();
          showToast(t("toast.demoCreated"));
        } catch (e) {
          showToast(t("toast.demoFailed", { err: String(e) }));
        }
      } else {
        // 不生成演示案例：写一份最小 __info.json，标记工作区已初始化，避免下次再询问
        try {
          await saveInfo(ws, {
            name: t("app.defaultWsName"),
            description: "",
            baseUrl: "",
            mockPort: 5050,
          });
        } catch {
          /* noop */
        }
      }
      await loadAll(ws);
      if (!create) showToast(t("toast.opened"));
    }
  };

  /** 导入 Postman Collection：自动新建分组，并把集合变量合并到环境变量 */
  const handleImportPostman = async () => {
    try {
      const result = await importPostman();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      if (result.vars > 0) {
        showToast(t("toast.importedPostman", { count: result.vars, env: result.env }));
      } else {
        showToast(t("toast.importedPostmanSimple"));
      }
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  /** 导入 OpenAPI (Swagger) 规范：自动新建分组并导入全部接口 */
  const handleImportOpenApi = async () => {
    try {
      const result = await importOpenApi();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedOpenApi", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  /** 导入 Apifox 项目：自动新建分组并导入全部接口 */
  const handleImportApifox = async () => {
    try {
      const result = await importApifox();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedApifox", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  /** 导入 Apipost 项目：自动新建分组并导入全部接口 */
  const handleImportApipost = async () => {
    try {
      const result = await importApipost();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedApipost", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  const handleImportRaml = async () => {
    try {
      const result = await importRaml();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedRaml", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  const handleImportWadl = async () => {
    try {
      const result = await importWadl();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedWadl", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  const handleImportHar = async () => {
    try {
      const result = await importHar();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedHar", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  const handleImportYapi = async () => {
    try {
      const result = await importYapi();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedYapi", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  const handleImportEolink = async () => {
    try {
      const result = await importEolink();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedEolink", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  const handleImportInsomnia = async () => {
    try {
      const result = await importInsomnia();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedInsomnia", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
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
      showToast(t("toast.markdownFailed", { err: String(e) }));
    }
  };

  /** 保存 Markdown / HTML 文件到用户选择的目录 */
  const handleExportMarkdown = async (format: "md" | "html") => {
    if (!mdView) return;
    setExporting(true);
    try {
      const saved = await exportApiMarkdown(mdView.node.path, format, settings.htmlNav);
      if (saved) showToast(t("toast.savedTo", { path: saved }));
    } catch (e) {
      showToast(t("toast.saveFailed", { err: String(e) }));
    } finally {
      setExporting(false);
    }
  };

  /** 导入 Markdown 接口文档：自动新建分组并导入全部接口 */
  const handleImportMarkdown = async () => {
    try {
      const result = await importMarkdown();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      showToast(t("toast.importedMarkdown", { count: result.count }));
      void reloadMockIfRunning();
    } catch (e) {
      showToast(t("toast.importFailed", { err: String(e) }));
    }
  };

  /** 导出选中接口/分组为 Postman / OpenAPI / Docsify / Markdown / HTML 格式 */
  const handleExport = async (paths: string[], format: ExportFormat) => {
    setExporting(true);
    try {
      const saved = await exportSelection(paths, format, settings.htmlNav);
      if (!saved) return; // 用户取消
      const kind = format === "docsify" ? t("export.kindDir") : t("export.kindFile");
      showToast(t("toast.exported", { kind, path: saved }));
      setExportOpen(false);
      setExportPreselect(undefined);
    } catch (e) {
      showToast(t("toast.exportFailed", { err: String(e) }));
    } finally {
      setExporting(false);
    }
  };

  /** 打开导出弹窗（可选预选某个节点） */
  const openExport = (node?: TreeNode) => {
    setExportPreselect(node ? [node.path] : undefined);
    setExportOpen(true);
  };

  /** 同步（git pull / svn update） */
  const handleVcsSync = async () => {
    if (!vcs || !settings.syncRemote) return; // 按钮仅在开启同步远程时显示
    try {
      const out = await vcsSync(settings.syncRemote);
      showToast(out.split("\n")[0] || t("toast.synced"));
    } catch (e) {
      setNotify({ title: t("notify.syncFailed"), body: String(e) });
    }
  };

  /** 提交并 Push 远程（未开启同步远程时只提交） */
  const handleVcsCommitPush = async () => {
    if (!vcs) return;
    try {
      const out = await vcsCommitPush(settings.syncRemote);
      showToast(out.split("\n")[0] || t("toast.committed"));
    } catch (e) {
      setNotify({ title: t("notify.commitFailed"), body: String(e) });
    }
  };

  /** 左右分栏拖动调整宽度 */
  const startResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = sidebarWidthRef.current;
    const onMove = (ev: MouseEvent) => {
      const w = Math.min(640, Math.max(200, startW + ev.clientX - startX));
      sidebarWidthRef.current = w;
      setSidebarWidth(w);
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      localStorage.setItem("sidebar-width", String(sidebarWidthRef.current));
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  /** 编辑器 / 响应上下分栏拖动调整高度（拖动中直接操作 DOM + rAF 合并帧，避免每个 mousemove 触发 React 重渲染而卡顿） */
  const startVResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startRatio = editorRatioRef.current; // 拖动开始时的比例（基线，避免累计放大导致闪跳）
    const contentEl = (e.currentTarget as HTMLElement).parentElement as HTMLElement;
    const contentH = contentEl.clientHeight;
    const editorEl = contentEl.querySelector<HTMLElement>(".editor");
    // 预留分隔条 + 响应面板最小高度
    const maxRatio = Math.max(0.2, (contentH - 165) / contentH);
    let lastY = startY;
    let raf = 0;
    const onMove = (ev: MouseEvent) => {
      lastY = ev.clientY;
      if (raf) return; // 已有一帧待执行，丢弃中间事件
      raf = requestAnimationFrame(() => {
        raf = 0;
        const ratio = Math.min(
          maxRatio,
          Math.max(0.2, startRatio + (lastY - startY) / contentH)
        );
        editorRatioRef.current = ratio;
        if (editorEl) editorEl.style.height = `${ratio * 100}%`;
      });
    };
    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      if (raf) cancelAnimationFrame(raf);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      setEditorRatio(editorRatioRef.current);
      localStorage.setItem("editor-ratio", String(editorRatioRef.current));
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
  };

  // ===== WebSocket 持久连接与交互记录 =====
  const wsRef = useRef<WebSocket | null>(null);
  const wsQueueRef = useRef<string[]>([]);
  /** 当前 WS 连接地址（写请求历史用） */
  const wsUrlRef = useRef("");
  /** 最近一次发送的消息文本（保存示例用） */
  const wsLastSentRef = useRef<string>("");
  /** 最近一次收到的消息文本（保存示例用） */
  const wsLastRecvRef = useRef<string>("");
  /** 最近一次发送→回显的耗时（保存示例用） */
  const wsLastRtRef = useRef(0);
  /** 待回显的请求（收到回显/连接错误时写入请求历史，一次发送对应一条记录） */
  const wsPendingRef = useRef<{ text: string; time: number } | null>(null);
  const [wsConnected, setWsConnected] = useState(false);
  const [wsConnecting, setWsConnecting] = useState(false);
  const [wsEntries, setWsEntries] = useState<WsLogEntry[]>([]);

  const appendWsEntry = (dir: WsLogEntry["dir"], text: string) =>
    setWsEntries((prev) => [...prev, { dir, text, time: Date.now() }]);

  const doWsSend = (ws: WebSocket, text: string) => {
    try {
      ws.send(text);
      appendWsEntry("sent", text);
      wsLastSentRef.current = text;
      wsPendingRef.current = { text, time: Date.now() };
    } catch (e) {
      appendWsEntry("error", `${t("app.wsSendFailed")}：${e}`);
    }
  };

  /** 收到回显或连接中断时，将待记录的请求写入 .history 日志（仅记录「发送过消息」的交互） */
  const flushWsPending = (recv: string, error?: string) => {
    const pend = wsPendingRef.current;
    if (!pend) return;
    wsPendingRef.current = null;
    wsLastRtRef.current = Date.now() - pend.time;
    saveHistory({
      method: "WS",
      url: wsUrlRef.current,
      apiUuid: api?.uuid,
      apiName: api?.name,
      reqHeaders: [], // 浏览器 WebSocket API 无法自定义请求头
      reqBody: pend.text,
      ok: !error,
      status: 0,
      statusText: "",
      respHeaders: [],
      respBody: recv,
      timeMs: wsLastRtRef.current,
      size: recv.length,
      error,
    }).catch((e) => console.error("保存 WebSocket 请求历史失败", e));
  };

  /** 关闭当前 WebSocket 连接并清空交互记录 */
  const closeWsConnection = () => {
    const ws = wsRef.current;
    if (ws) {
      try { ws.close(); } catch {}
    }
    wsRef.current = null;
    wsQueueRef.current = [];
    setWsConnected(false);
    setWsConnecting(false);
    setWsEntries([]);
  };

  /** 建立 WebSocket 连接（首次点「发送」时触发），此后保持长连接 */
  const openWsConnection = (url: string) => {
    setWsConnecting(true);
    let ws: WebSocket;
    try {
      // 浏览器 WebSocket API 无法携带自定义请求头：
      // 不能把 Header 值当作子协议传入（服务器不回显子协议时浏览器会判定握手失败）
      ws = new WebSocket(url);
    } catch (e) {
      setWsConnecting(false);
      appendWsEntry("error", `${t("app.wsConnectError")}：${e}`);
      return;
    }
    wsRef.current = ws;
    // 连接建立后服务器可能先推送一条欢迎/问候消息（demo 服务器会发送 type:welcome），
    // 不把它当作「发送消息」的回显写入请求历史
    let firstRecv = true;
    ws.onopen = () => {
      setWsConnecting(false);
      setWsConnected(true);
      appendWsEntry("info", t("resp.wsConnected"));
      // 连接前的待发消息一次性补发
      const q = wsQueueRef.current;
      wsQueueRef.current = [];
      for (const m of q) doWsSend(ws, m);
    };
    ws.onmessage = (ev) => {
      const d = ev.data;
      const finish = (s: string) => {
        appendWsEntry("recv", s);
        wsLastRecvRef.current = s;
        if (firstRecv) {
          firstRecv = false; // 首条消息为服务器主动推送，仅展示不配对
        } else {
          flushWsPending(s);
        }
      };
      if (d instanceof Blob) {
        d.text().then(finish);
      } else {
        finish(typeof d === "string" ? d : String(d));
      }
    };
    ws.onerror = () => {
      setWsConnected(false);
      setWsConnecting(false);
      appendWsEntry("error", t("app.wsConnectError"));
      flushWsPending("", t("app.wsConnectError"));
    };
    ws.onclose = () => {
      setWsConnected(false);
      setWsConnecting(false);
      if (wsRef.current === ws) wsRef.current = null;
      flushWsPending("", t("app.wsConnectError"));
    };
  };

  /** 发送 WebSocket 消息：复用已建立的连接；无连接时先建连再发送 */
  const handleWsSend = (url: string, body: BodyData) => {
    wsUrlRef.current = url;
    const text = body.raw || "";
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      doWsSend(ws, text);
    } else if (ws && ws.readyState === WebSocket.CONNECTING) {
      wsQueueRef.current.push(text); // 连接中：排队待发
    } else {
      wsQueueRef.current = [text];
      openWsConnection(url); // 无连接：建立连接后再发
    }
  };

  // 切换接口 / 组件卸载时关闭 WS 连接
  useEffect(() => {
    closeWsConnection();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api?.uuid]);
  useEffect(() => () => closeWsConnection(), []);


  const handleSend = async () => {
    if (!api) return;
    setSending(true);
    try {
      // 当前激活环境的变量表
      const activeEnv = envs.environments.find((e) => e.name === envs.active);
      const vars: Record<string, string> = {};
      for (const v of activeEnv?.variables || []) {
        if (v.enabled && v.key.trim())
          vars[v.key.trim()] = v.value.trim() ? v.value : v.defaultValue;
      }
      const sub = (s: string) =>
        s.replace(/\{\{([^{}]+)\}\}/g, (m, k: string) => vars[k.trim()] ?? m);

      const headers = api.headers
        .filter((h) => h.enabled && h.key.trim())
        .map((h) => ({ ...h, key: sub(h.key), value: sub(h.value) }));
      // XML 模式：未手动设置 Content-Type 时默认 application/xml（避免后端默认按 JSON 处理）
      if (
        api.body.mode === "xml" &&
        !headers.some((h) => h.key.toLowerCase() === "content-type")
      ) {
        headers.push({ key: "Content-Type", value: "application/xml; charset=utf-8", enabled: true, description: "" });
      }
      let url = sub(api.url || rootInfo.baseUrl + api.path);
      // 替换路径参数（多个示例值逗号分隔，发送时取第一个）；
      // 仅替换单大括号 {变量名}，不触碰 {{变量名}} 全局环境变量
      // WebSocket 不使用路径参数（无 Path 页签），跳过替换
      if (api.protocol !== "websocket") {
        for (const p of api.params.filter((x) => x.enabled && x.key)) {
          const v = p.value.split(",")[0].trim();
          const rx = new RegExp(`(?<!\\{)\\{${escapeRe(p.key)}\\}(?!\\})`, "g");
          url = url.replace(rx, encodeURIComponent(sub(v)));
        }
      }
      // URL 校验：空地址 / 缺少协议前缀 / 存在未替换的 {{变量}}
      if (!url.trim()) {
        throw new Error(t("app.urlEmpty"));
      }
      if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(url)) {
        url = "http://" + url;
      }
      const unresolved = [...url.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)].map((m) => m[0]);
      if (unresolved.length > 0) {
        throw new Error(t("app.envUnresolved", { names: unresolved.join(t("app.envJoin")) }));
      }
      // 拼接 query
      const qs = api.query
        .filter((q) => q.enabled && q.key)
        .map((q) => `${encodeURIComponent(sub(q.key))}=${encodeURIComponent(sub(q.value))}`)
        .join("&");
      if (qs) url += (url.includes("?") ? "&" : "?") + qs;

      // WebSocket：持久连接发送，交互记录展示在响应区（不设置 HTTP 式 result）
      if (api.protocol === "websocket") {
        handleWsSend(url, api.body);
        setLastRequest({ method: "WS", url, headers, body: api.body.raw, timeoutMs: 30000 });
        setLastApiSnapshot(api);
        setSending(false); // WS 发送为异步短操作，立即复位按钮状态
        return;
      }

      // 表单：含文件字段时走 multipart（req.form），否则拼 urlencoded body
      const formRows = api.body.form.filter((f) => f.enabled && f.key);
      // 二进制模式：未选择文件时直接报错
      if (api.body.mode === "binary" && !api.body.binaryPath.trim()) {
        throw new Error(t("app.bodyBinaryEmpty"));
      }
      const body =
        api.body.mode === "form"
          ? formRows
              .map((f) => `${encodeURIComponent(sub(f.key))}=${encodeURIComponent(sub(f.value))}`)
              .join("&")
          : api.body.mode === "raw" || api.body.mode === "json" || api.body.mode === "xml"
          ? sub(api.body.raw)
          : undefined;
      const hasFile = api.body.mode === "form" && formRows.some((f) => f.isFile);

      const req: HttpRequestData = {
        method: api.method,
        url,
        headers,
        body: hasFile ? undefined : body,
        bodyFile: api.body.mode === "binary" ? api.body.binaryPath.trim() : undefined,
        form: hasFile
          ? formRows.map((f) => ({ ...f, key: sub(f.key), value: sub(f.value) }))
          : undefined,
        timeoutMs: 30000,
      };
      const res = await sendRequest(req);
      setResponse(res);
      setLastRequest(req);
      setLastApiSnapshot(api);
      // 每次请求都保存到 .history 目录（按天分文件）
      try {
        await saveHistory({
          method: req.method,
          url: req.url,
          apiUuid: api?.uuid,
          apiName: api?.name,
          reqHeaders: req.headers.map((h) => [h.key, h.value]),
          reqBody: req.body,
          ok: res.ok,
          status: res.status,
          statusText: res.statusText,
          respHeaders: res.headers,
          respBody: res.body,
          timeMs: res.timeMs,
          size: res.size,
          error: res.error,
        });
      } catch (e) {
        console.error("保存请求历史失败", e);
      }
    } catch (e) {
      setResponse({ ok: false, status: 0, statusText: "", headers: [], body: "", timeMs: 0, size: 0, url: "", error: String(e) });
    } finally {
      setSending(false);
    }
  };

  // 将最近一次请求与响应保存为示例 -> 工作区 .examples/<接口uuid>/<示例名称hash值>.json
  const handleSaveExample = async (name: string) => {
    if (!api || !lastRequest) return;
    const isWs = api.protocol === "websocket";
    if (!isWs && !response) return;
    const snap = lastApiSnapshot || api;
    try {
      // 从最终请求 URL 解析出 query 参数（用户在 URL 里直接写的 ?a=1&b=2 也要收录）
      const urlQuery: [string, string][] = [];
      const qi = lastRequest.url.indexOf("?");
      if (qi >= 0) {
        for (const part of lastRequest.url.slice(qi + 1).split("&")) {
          if (!part) continue;
          try {
            const eq = part.indexOf("=");
            const k = eq >= 0 ? decodeURIComponent(part.slice(0, eq)) : decodeURIComponent(part);
            const v = eq >= 0 ? decodeURIComponent(part.slice(eq + 1)) : "";
            if (k) urlQuery.push([k, v]);
          } catch {
            // 编码异常的参数跳过
          }
        }
      }
      // 表格 query 优先，URL 中表格没有的参数补充进来（避免遗漏 URL 上直接写的参数）
      const reqQuery: [string, string][] = snap.query
        .filter((q) => q.enabled && q.key.trim())
        .map((q) => [q.key, q.value]);
      const seen = new Set(reqQuery.map(([k]) => k));
      for (const [k, v] of urlQuery) {
        if (!seen.has(k)) reqQuery.push([k, v]);
      }
      if (isWs) {
        // WebSocket：保存最近一次发送的消息与收到的回显
        await saveExample(api.uuid || crypto.randomUUID(), name, {
          name,
          time: Math.floor(Date.now() / 1000),
          method: "WS",
          url: lastRequest.url,
          reqHeaders: [], // 浏览器 WebSocket API 无法自定义请求头
          reqPath: [],
          reqQuery,
          reqBody: wsLastSentRef.current || undefined,
          status: 0,
          statusText: "",
          respHeaders: [],
          respBody: wsLastRecvRef.current,
          timeMs: wsLastRtRef.current,
          size: wsLastRecvRef.current.length,
        });
      } else {
        if (!response) return; // HTTP 需要最近一次响应
        await saveExample(api.uuid || crypto.randomUUID(), name, {
          name,
          time: Math.floor(Date.now() / 1000),
          method: lastRequest.method,
          url: lastRequest.url,
          reqHeaders: lastRequest.headers.map((h) => [h.key, h.value]),
          reqPath: snap.params
            .filter((p) => p.enabled && p.key.trim())
            .map((p) => [p.key, p.value]),
          reqQuery,
          reqBody: lastRequest.body,
          status: response.status,
          statusText: response.statusText,
          respHeaders: response.headers,
          respBody: response.body,
          timeMs: response.timeMs,
          size: response.size,
          error: response.error || undefined,
        });
      }
      showToast(t("toast.exampleSaved", { name }));
      setExampleVersion((v) => v + 1);
    } catch (e) {
      showToast(t("toast.saveExampleFailed", { err: String(e) }));
    }
  };

  // 接口说明失焦后自动保存
  const handleAutoSave = useCallback(async () => {
    if (!dirty || !api || !selectedPath) return;
    try {
      await saveApi(selectedPath, api);
      setDirty(false);
      showToast(t("toast.saved"));
    } catch (e) {
      showToast(t("toast.saveFailed", { err: String(e) }));
    }
  }, [dirty, api, selectedPath, showToast]);

  // 保存接口新版本 -> 工作区 .version/<uuid>/<名称>.<版本号>.json
  const handleSaveVersion = async () => {
    if (!api) return;
    try {
      let data = api;
      if (!data.uuid) {
        data = { ...data, uuid: crypto.randomUUID() };
        // 先持久化 uuid 到主文件，避免后续版本目录分裂
        if (selectedPath) {
          await saveApi(selectedPath, data);
          setDirty(false);
        }
      }
      // 与最新历史版本对比：无改动则提示已是最新版本，不再重复保存
      const versions = await listVersions(data.uuid);
      if (versions.length > 0) {
        const latest = await readApiVersion(versions[0].path);
        if (JSON.stringify(data) === JSON.stringify(JSON.parse(latest))) {
          showToast(t("toast.alreadyLatest"));
          return;
        }
      }
      const rel = await saveApiVersion(data);
      showToast(t("toast.savedVersion", { rel }));
      void refreshVersion(data.uuid);
    } catch (e) {
      showToast(t("toast.saveVersionFailed", { err: String(e) }));
    }
  };

  /** 版本恢复成功后：重新加载接口内容并刷新左侧树/版本号 */
  const handleVersionRestored = async (path: string, version: number) => {
    try {
      const data = await readApi(path);
      if (!data.uuid) data.uuid = crypto.randomUUID();
      setSelectedPath(path);
      setApi(data);
      setDirty(false);
      setResponse(null);
      void refreshVersion(data.uuid);
      void reloadTree();
      setVersionModal(null);
      showToast(t("version.restored", { version }));
    } catch (e) {
      showToast(t("toast.saveFailed", { err: String(e) }));
    }
  };

  const toggleMock = async () => {
    try {
      if (mock.running) {
        setMock(await mockStop());
        showToast(t("mock.stopped"));
      } else {
        const port = settings.mockPort || 5050;
        const s = await mockStart(port);
        setMock(s);
        if (s.routeCount > 0) {
          showToast(t("mock.startedWithRoutes", { port, count: s.routeCount }));
        } else {
          showToast(t("mock.noRoutes"));
        }
      }
    } catch (e) {
      showToast(t("mock.failed", { err: String(e) }));
    }
  };

  /** 新增/复制/导入接口后，若 Mock 服务运行中则热重载路由 */
  const reloadMockIfRunning = async () => {
    if (!mock.running) return;
    try {
      const s = await mockReload();
      setMock(s);
    } catch {
      /* noop */
    }
  };

  const handleEnvSwitch = async (active: string) => {
    const next = { ...envs, active };
    setEnvs(next);
    updateTrayEnv(active || "").catch(() => {});
    try {
      await saveEnv(next);
      if (mock.running) {
        const m = await mockReload();
        setMock(m);
      }
      showToast(active ? t("toast.envSwitched", { name: active }) : t("toast.noEnv"));
    } catch (e) {
      showToast(t("toast.envSwitchFailed", { err: String(e) }));
    }
  };

  const handleSaveEnv = async (data: EnvStore) => {
    try {
      await saveEnv(data);
      setEnvs(data);
      updateTrayEnv(data.active || "").catch(() => {});
      setEnvModal(false);
      if (mock.running) {
        const m = await mockReload();
        setMock(m);
      }
      showToast(data.active ? t("toast.envSaved", { name: data.active }) : t("toast.envSavedNone"));
    } catch (e) {
      showToast(t("toast.saveEnvFailed", { err: String(e) }));
    }
  };

  // 主页面直接编辑当前环境集的变量值
  const activeEnv = envs.environments.find((e) => e.name === envs.active);
  const handleSaveEnvValues = async (variables: EnvVariable[]) => {
    if (!activeEnv) return;
    const next: EnvStore = {
      ...envs,
      environments: envs.environments.map((e) =>
        e.name === activeEnv.name ? { ...e, variables } : e
      ),
    };
    setEnvs(next);
    try {
      await saveEnv(next);
      updateTrayEnv(next.active || "").catch(() => {});
      if (mock.running) {
        setMock(await mockReload());
      }
      showToast(t("toast.envValuesSaved", { name: activeEnv.name }));
    } catch (e) {
      showToast(t("toast.saveEnvValuesFailed", { err: String(e) }));
    }
    setEnvValue(false);
  };

  // ---------- 弹窗操作 ----------
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
      showToast(t("toast.readInfoFailed", { err: String(e) }));
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
      setSelectedPath(path);
      setApi(data);
      setDirty(false);
      setResponse(null);
      void refreshVersion(data.uuid);
      void reloadMockIfRunning();
      showToast(t("toast.createdApi", { name }));
    } catch (e) {
      showToast(t("toast.failed", { err: String(e) }));
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
      showToast(t("toast.createdFolder", { name }));
    } catch (e) {
      showToast(t("toast.failed", { err: String(e) }));
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
      showToast(t("toast.renamed"));
    } catch (e) {
      showToast(t("toast.renameFailed", { err: String(e) }));
    }
  };

  const handleCopy = async (node: TreeNode) => {
    try {
      const p = await copyEntry(node.path);
      await reloadTree();
      void reloadMockIfRunning();
      showToast(t("toast.copied", { name: p }));
    } catch (e) {
      showToast(t("toast.failed", { err: String(e) }));
    }
  };

  const doDelete = async () => {
    if (!modal?.target) return;
    try {
      await deleteEntry(modal.target.path);
      setModal(null);
      if (selectedPath === modal.target.path) {
        setSelectedPath(null);
        setApi(null);
        setResponse(null);
      }
      await reloadTree();
      showToast(t("toast.deleted"));
    } catch (e) {
      showToast(t("toast.deleteFailed", { err: String(e) }));
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
      showToast(t("toast.saved"));
    } catch (e) {
      showToast(t("toast.saveFailed", { err: String(e) }));
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
      showToast(t("toast.readVersionsFailed", { err: String(e) }));
    }
  };

  // 修改工作区名称（写入根目录 __info.json）
  const saveWorkspaceName = async (name: string) => {
    const n = name.trim();
    if (!n) {
      showToast(t("toast.wsNameEmpty"));
      return;
    }
    if (!workspace) return;
    try {
      await saveInfo(workspace, { name: n });
      setRootInfo((prev) => ({ ...prev, name: n }));
      await reloadTree();
      showToast(t("toast.wsNameUpdated"));
    } catch (e) {
      showToast(t("toast.saveFailed", { err: String(e) }));
    }
  };

  // 设置即时生效：每次修改直接持久化，无需点保存
  const handleSaveSettings = async (s: AppSettings) => {
    setSettings(s);
    try {
      await saveSettings(s);
    } catch (e) {
      showToast(t("toast.saveSettingsFailed", { err: String(e) }));
    }
  };

  // 拖拽移动接口/目录到其他目录
  const handleMove = async (srcPath: string, dstDir: string) => {
    try {
      const newPath = await moveEntry(srcPath, dstDir);
      await reloadTree();
      setSelectedPath((prev) => {
        if (prev === srcPath) return newPath;
        if (prev && prev.startsWith(srcPath + "/")) return null; // 目录被移动，内部选中项路径已失效
        return prev;
      });
      showToast(t("toast.moved"));
    } catch (e) {
      showToast(t("toast.moveFailed", { err: String(e) }));
    }
  };

  // 标记 / 取消标记“已废弃”（接口或分组），成功后在左侧树与当前编辑的接口上即时生效
  const handleToggleDeprecated = async (node: TreeNode) => {
    try {
      const now = await toggleDeprecated(node.path);
      await reloadTree();
      // 若是当前正编辑的接口，同步刷新编辑区内容
      if (node.kind === "api" && selectedPath === node.path) {
        const data = await readApi(node.path);
        setApi(data);
        setDirty(false);
      }
      showToast(
        now
          ? t("toast.markedDeprecated", { name: node.name })
          : t("toast.unmarkedDeprecated", { name: node.name })
      );
    } catch (e) {
      showToast(t("toast.failed", { err: String(e) }));
    }
  };

  const baseUrl = rootInfo.baseUrl || "";

  // ---------- 渲染 ----------
  if (!workspace) {
    return (
      <Landing
        version={version}
        updateInfo={updateInfo}
        recent={recent}
        recentLimit={recentLimit}
        onPickWorkspace={() => void handlePickWorkspace()}
        onOpenRecent={(ws) => void handleOpenRecent(ws)}
        onOpenUpdate={() => setShowUpdateModal(true)}
      />
    );
  }

  return (
    <div className="app">
      <AppToolbar
        workspace={workspace}
        version={version}
        updateInfo={updateInfo}
        recent={recent}
        recentLimit={recentLimit}
        envs={envs}
        mock={mock}
        settings={settings}
        onPickWorkspace={() => void handlePickWorkspace()}
        onOpenRecent={(ws) => void handleOpenRecent(ws)}
        onSwitchEnv={(name) => void handleEnvSwitch(name)}
        onOpenEnvValue={() => setEnvValue(true)}
        onOpenEnvModal={() => setEnvModal(true)}
        onToggleMock={() => void toggleMock()}
        onRefresh={async () => {
          await reloadTree(true);
          showToast(t("toast.refreshed"));
        }}
        onOpenUpdate={() => setShowUpdateModal(true)}
      />
      <div className="main">
        <Sidebar
          width={sidebarWidth}
          tree={tree}
          loading={treeLoading}
          selectedPath={selectedPath}
          view={view}
          onSwitchView={switchView}
          onSelect={selectNode}
          onNewApi={(parent) => openModal("newApi", parent)}
          onNewFolder={(parent) => openModal("newFolder", parent)}
          onRename={(node) => openModal("rename", "", node)}
          onCopy={handleCopy}
          onDelete={(node) => openModal("delete", "", node)}
          onToggleDeprecated={(node) => void handleToggleDeprecated(node)}
          onEditInfo={(node) => openInfoModal(node)}
          onVersions={openVersions}
          onStats={setStatsNode}
          onOpenSettings={() => setSettingsOpen(true)}
          onImportPostman={() => void handleImportPostman()}
          onImportOpenApi={() => void handleImportOpenApi()}
          onImportMarkdown={() => void handleImportMarkdown()}
          onImportApifox={() => void handleImportApifox()}
          onImportApipost={() => void handleImportApipost()}
          onImportRaml={() => void handleImportRaml()}
          onImportWadl={() => void handleImportWadl()}
          onImportHar={() => void handleImportHar()}
          onImportYapi={() => void handleImportYapi()}
          onImportEolink={() => void handleImportEolink()}
          onImportInsomnia={() => void handleImportInsomnia()}
          settings={settings}
          onViewMarkdown={(node) => void handleViewMarkdown(node)}
          onExport={() => openExport()}
          onExportNode={(node) => openExport(node)}
          vcs={null} // 同步远程功能暂时隐藏（后端命令保留，恢复时改回 vcs && settings.syncRemote ? vcs : null）
          onVcsSync={() => void handleVcsSync()}
          onVcsCommitPush={() => void handleVcsCommitPush()}
          onMove={handleMove}
          enableVersion={settings.enableVersion}
          historyRecords={history.records}
          historyDays={history.days}
          historyLoading={history.loading}
          historyHasMore={history.hasMore}
          historySelected={history.selectedId}
          historyTotal={history.totalCount}
          onHistorySelect={(id) => void history.select(id)}
          onHistoryLoadMore={() => void history.loadPage(history.offset)}
          onHistoryReload={history.reload}
          onHistoryClear={() => void history.clearAll()}
          historyDiffMode={history.diffMode}
          historyDiffIds={history.diffIds}
          historyDiffError={history.diffError}
          onHistoryToggleDiffMode={history.toggleDiffMode}
          onHistoryToggleDiffSelect={history.toggleDiffSelect}
          onHistoryStartDiff={() => void history.startDiff()}
        />
        <RightPane
          view={view}
          api={api}
          historyDetail={history.detail}
          historyDetailLoading={history.detailLoading}
          historyDiff={history.diffPair}
          historyDiffLoading={history.diffLoading}
          onHistoryDiffExit={history.exitDiff}
          baseUrl={baseUrl}
          currentVersion={currentVersion}
          exampleVersion={exampleVersion}
          enableVersion={settings.enableVersion}
          enableCodegen={settings.enableCodegen}
          enableMock={settings.enableMock}
          codegenLang={settings.codegenLang}
          sending={sending}
          hideResponse={hideResponse}
          editorRatio={editorRatio}
          response={response}
          wsConnected={wsConnected}
          wsConnecting={wsConnecting}
          wsEntries={wsEntries}
          onApiChange={(a) => {
            setApi(a);
            setDirty(true);
          }}
          onSend={handleSend}
          onSaveExample={handleSaveExample}
          onSaveVersion={handleSaveVersion}
          onCommit={handleAutoSave}
          onTabChange={(t) => setHideResponse(["response", "mock", "desc", "doc", "code", "examples"].includes(t))}
          onStartVResize={startVResize}
          onResetRatio={() => {
            setEditorRatio(0.45);
            editorRatioRef.current = 0.45;
            localStorage.setItem("editor-ratio", "0.45");
          }}
          onWsDisconnect={closeWsConnection}
          onResizeStart={startResize}
          onResizeReset={() => {
            setSidebarWidth(310);
            sidebarWidthRef.current = 310;
            localStorage.setItem("sidebar-width", "310");
          }}
          resizeTip={t("app.resizeSidebarTip")}
          onEmptyContextMenu={(e) => {
            e.preventDefault();
            setEmptyMenu({
              x: Math.min(e.clientX, window.innerWidth - 190),
              y: Math.min(e.clientY, window.innerHeight - 160),
            });
          }}
        />
      </div>

      <AppModals
        toast={toast}
        notify={notify}
        emptyMenu={emptyMenu}
        versionModal={versionModal}
        statsNode={statsNode}
        showUpdateModal={showUpdateModal}
        updateInfo={updateInfo}
        mdView={mdView}
        exportOpen={exportOpen}
        exportPreselect={exportPreselect}
        exporting={exporting}
        tree={tree}
        defaultFormat={settings.exportFormat}
        settings={settings}
        settingsOpen={settingsOpen}
        appVersion={version}
        workspaceName={
          rootInfo.name ||
          (workspace ? workspace.split(/[\\/]/).filter(Boolean).pop() || workspace : "")
        }
        envModal={envModal}
        envs={envs}
        envValue={envValue}
        activeEnv={activeEnv}
        modal={modal}
        modalText={modalText}
        modalProtocol={modalProtocol}
        infoForm={infoForm}
        demoCreate={demoCreate}
        workspace={workspace}
        onCloseNotify={() => setNotify(null)}
        onEmptyMenuAction={(action) => {
          openModal(action === "newApi" ? "newApi" : "newFolder", "");
          setEmptyMenu(null);
        }}
        onCloseVersionModal={() => setVersionModal(null)}
        onVersionRestored={handleVersionRestored}
        onCloseStats={() => setStatsNode(null)}
        onCloseUpdate={() => setShowUpdateModal(false)}
        onCloseMarkdown={() => setMdView(null)}
        onExportMarkdown={handleExportMarkdown}
        onCloseExport={() => {
          setExportOpen(false);
          setExportPreselect(undefined);
        }}
        onExport={handleExport}
        onCloseSettings={() => setSettingsOpen(false)}
        onSaveWorkspaceName={saveWorkspaceName}
        onSaveSettings={handleSaveSettings}
        onCloseEnvModal={() => setEnvModal(false)}
        onSaveEnv={handleSaveEnv}
        onCloseEnvValue={() => setEnvValue(false)}
        onSaveEnvValues={handleSaveEnvValues}
        onCloseModal={() => setModal(null)}
        onModalTextChange={setModalText}
        onModalProtocolChange={setModalProtocol}
        onInfoFormChange={setInfoForm}
        onDemoCreateChange={setDemoCreate}
        onDoNewApi={() => void doNewApi()}
        onDoNewFolder={() => void doNewFolder()}
        onDoRename={() => void doRename()}
        onDoDelete={() => void doDelete()}
        onDoSaveInfo={() => void doSaveInfo()}
        onCloseDemoModal={(create) => void closeDemoModal(create)}
      />
    </div>
  );
}
