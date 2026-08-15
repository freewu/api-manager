import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createApi,
  createDemo,
  createFolder,
  deleteEntry,
  getAppVersion,
  getWorkspace,
  importPostman,
  listVersions,
  loadSettings,
  mockReload,
  mockStart,
  mockStatus,
  mockStop,
  pickWorkspace,
  readApi,
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
  updateTrayEnv,
  vcsCommitPush,
  vcsInfo,
  vcsSync,
  hasWorkspaceInfo,
} from "./commands";
import { Editor } from "./components/Editor";
import { EnvModal } from "./components/EnvModal";
import { EnvValueModal } from "./components/EnvValueModal";
import { HistoryDetail } from "./components/HistoryDetail";
import { AppView } from "./components/Sidebar";
import { useHistory } from "./hooks/useHistory";
import { Modal } from "./components/Modal";
import { Response } from "./components/Response";
import { SettingsModal } from "./components/SettingsModal";
import { Sidebar } from "./components/Sidebar";
import { StatsModal } from "./components/StatsModal";
import { VersionModal } from "./components/VersionModal";
import {
  ApiFile,
  AppSettings,
  EnvStore,
  EnvVariable,
  HttpRequestData,
  HttpResult,
  InfoJson,
  MockStatus,
  TreeNode,
  VersionInfo,
  defaultSettings,
  emptyEnv,
} from "./types";

interface ModalState {
  type: "newApi" | "newFolder" | "rename" | "delete" | "info" | "demo";
  parent: string;
  target?: TreeNode;
}

interface InfoForm {
  name: string;
  description: string;
}

const emptyInfoForm = (): InfoForm => ({
  name: "",
  description: "",
});

export default function App() {
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [rootInfo, setRootInfo] = useState<InfoJson>({});
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [api, setApi] = useState<ApiFile | null>(null);
  const [dirty, setDirty] = useState(false);
  const [response, setResponse] = useState<HttpResult | null>(null);
  const [sending, setSending] = useState(false);
  const [mock, setMock] = useState<MockStatus>({ running: false, routeCount: 0 });
  const [vcs, setVcs] = useState<"git" | "svn" | null>(null);
  const [modal, setModal] = useState<ModalState | null>(null);
  const [modalText, setModalText] = useState("");
  const [infoForm, setInfoForm] = useState<InfoForm>(emptyInfoForm());
  const [toast, setToast] = useState<string | null>(null);
  /** 右下角持久弹窗（不自动消失），用于同步/提交等错误提示 */
  const [notify, setNotify] = useState<{ title: string; body: string } | null>(null);
  const [version, setVersion] = useState("");
  const [envs, setEnvs] = useState<EnvStore>(emptyEnv());
  const [envModal, setEnvModal] = useState(false);
  const [envValue, setEnvValue] = useState(false);
  const [versionModal, setVersionModal] = useState<{ api: ApiFile; versions: VersionInfo[] } | null>(null);
  const [statsNode, setStatsNode] = useState<TreeNode | null>(null);
  const [emptyMenu, setEmptyMenu] = useState<{ x: number; y: number } | null>(null);
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
    loadSettings().then(setSettings).catch(() => {});
    mockStatus().then(setMock).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
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

  async function loadAll(ws: string) {
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
    } else {
      setSelectedPath(null);
      setApi(null);
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

  async function reloadTree() {
    const t = await readTree();
    setTree(t);
  }

  // 递归更新树中指定路径节点的 method（修改 HTTP 方法时即时刷新左侧徽标）
  function patchNodeMethod(node: TreeNode, path: string, method: string): TreeNode {
    if (node.path === path) return { ...node, method };
    if (node.children) {
      return { ...node, children: node.children.map((c) => patchNodeMethod(c, path, method)) };
    }
    return node;
  }

  useEffect(() => {
    if (!api || !selectedPath) return;
    setTree((t) => (t ? patchNodeMethod(t, selectedPath, api.method) : t));
  }, [selectedPath, api?.method]);

  const selectNode = useCallback(
    async (node: TreeNode) => {
      if (dirty && api) {
        try {
          await saveApi(selectedPath!, api);
          showToast("已自动保存修改");
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
    },
    [dirty, api, selectedPath, showToast]
  );

  const handlePickWorkspace = async () => {
    try {
      const ws = await pickWorkspace();
      if (!ws) return;
      setWorkspace(ws);
      setResponse(null);
      if (!(await hasWorkspaceInfo())) {
        // 新的工作目录（没有 __info.json）：询问是否生成演示案例
        setDemoCreate(true);
        setModal({ type: "demo", parent: ws });
      } else {
        await loadAll(ws);
        showToast("已打开工作区");
      }
    } catch (e) {
      showToast("打开失败: " + e);
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
          showToast("已生成演示案例");
        } catch (e) {
          showToast("生成演示案例失败: " + e);
        }
      } else {
        // 不生成演示案例：写一份最小 __info.json，标记工作区已初始化，避免下次再询问
        try {
          await saveInfo(ws, {
            name: "我的 API 集合",
            description: "",
            baseUrl: "",
            mockPort: 5050,
          });
        } catch {
          /* noop */
        }
      }
      await loadAll(ws);
      if (!create) showToast("已打开工作区");
    }
  };

  /** 导入 Postman Collection：自动新建分组，并把集合变量合并到环境变量 */
  const handleImportPostman = async () => {
    try {
      const result = await importPostman();
      if (!result) return; // 用户取消
      await loadAll(workspace!);
      if (result.vars > 0) {
        showToast(
          `已导入 Postman Collection，${result.vars} 个变量已合并到环境变量集「${result.env}」`
        );
      } else {
        showToast("已导入 Postman Collection");
      }
    } catch (e) {
      showToast("导入失败: " + e);
    }
  };

  /** 同步（git pull / svn update） */
  const handleVcsSync = async () => {
    if (!vcs || !settings.syncRemote) return; // 按钮仅在开启同步远程时显示
    try {
      const out = await vcsSync(settings.syncRemote);
      showToast(out.split("\n")[0] || "同步完成");
    } catch (e) {
      setNotify({ title: "同步失败", body: String(e) });
    }
  };

  /** 提交并 Push 远程（未开启同步远程时只提交） */
  const handleVcsCommitPush = async () => {
    if (!vcs) return;
    try {
      const out = await vcsCommitPush(settings.syncRemote);
      showToast(out.split("\n")[0] || "提交完成");
    } catch (e) {
      setNotify({ title: "提交失败", body: String(e) });
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
      let url = sub(api.url || rootInfo.baseUrl + api.path);
      // 替换路径参数
      for (const p of api.params.filter((x) => x.enabled && x.key)) {
        url = url.replaceAll(`{${p.key}}`, encodeURIComponent(sub(p.value)));
      }
      // URL 校验：空地址 / 缺少协议前缀 / 存在未替换的 {{变量}}
      if (!url.trim()) {
        throw new Error("请求 URL 为空，请填写完整地址或设置 Base URL");
      }
      if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(url)) {
        url = "http://" + url;
      }
      const unresolved = [...url.matchAll(/\{\{\s*([^{}]+?)\s*\}\}/g)].map((m) => m[0]);
      if (unresolved.length > 0) {
        throw new Error(
          `URL 中存在未替换的环境变量: ${unresolved.join("、")}（请检查激活的环境变量，或直接修改 URL）`
        );
      }
      // 拼接 query
      const qs = api.query
        .filter((q) => q.enabled && q.key)
        .map((q) => `${encodeURIComponent(sub(q.key))}=${encodeURIComponent(sub(q.value))}`)
        .join("&");
      if (qs) url += (url.includes("?") ? "&" : "?") + qs;

      const body =
        api.body.mode === "form"
          ? api.body.form
              .filter((f) => f.enabled && f.key)
              .map((f) => `${encodeURIComponent(sub(f.key))}=${encodeURIComponent(sub(f.value))}`)
              .join("&")
          : api.body.mode === "raw" || api.body.mode === "json"
          ? sub(api.body.raw)
          : undefined;

      const req: HttpRequestData = {
        method: api.method,
        url,
        headers,
        body,
        timeoutMs: 30000,
      };
      const res = await sendRequest(req);
      setResponse(res);
      // 每次请求都保存到 .history 目录（按天分文件）
      try {
        await saveHistory({
          method: req.method,
          url: req.url,
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

  // 接口说明失焦后自动保存
  const handleAutoSave = useCallback(async () => {
    if (!dirty || !api || !selectedPath) return;
    try {
      await saveApi(selectedPath, api);
      setDirty(false);
      showToast("已保存");
    } catch (e) {
      showToast("保存失败: " + e);
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
      const rel = await saveApiVersion(data);
      showToast(`已保存新版本: ${rel}`);
    } catch (e) {
      showToast("保存版本失败: " + e);
    }
  };

  const toggleMock = async () => {
    try {
      if (mock.running) {
        setMock(await mockStop());
        showToast("Mock 服务已停止");
      } else {
        const port = settings.mockPort || 5050;
        setMock(await mockStart(port));
        showToast(`Mock 服务已启动: http://127.0.0.1:${port}`);
      }
    } catch (e) {
      showToast("Mock 操作失败: " + e);
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
      showToast(active ? `已切换环境: ${active}` : "已切换到无环境");
    } catch (e) {
      showToast("切换环境失败: " + e);
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
      showToast(data.active ? `环境已保存，当前: ${data.active}` : "环境已保存");
    } catch (e) {
      showToast("保存环境失败: " + e);
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
      showToast(`环境变量已保存：${activeEnv.name}`);
    } catch (e) {
      showToast("保存环境变量失败: " + e);
    }
    setEnvValue(false);
  };

  // ---------- 弹窗操作 ----------
  const openModal = (type: ModalState["type"], parent = "", target?: TreeNode) => {
    setModalText(target?.name || (type === "newApi" ? "未命名接口" : type === "newFolder" ? "新分组" : ""));
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
      showToast("读取信息失败: " + e);
    }
  };

  const doNewApi = async () => {
    if (!modal) return;
    const name = modalText.trim() || "未命名接口";
    try {
      const dir = modal.parent || workspace!;
      const path = await createApi(dir, name);
      setModal(null);
      await reloadTree();
      const data = await readApi(path);
      setSelectedPath(path);
      setApi(data);
      setDirty(false);
      setResponse(null);
      showToast(`已创建接口: ${name}`);
    } catch (e) {
      showToast("创建失败: " + e);
    }
  };

  const doNewFolder = async () => {
    if (!modal) return;
    const name = modalText.trim() || "新分组";
    try {
      const parent = modal.parent || workspace!;
      await createFolder(parent, name);
      setModal(null);
      await reloadTree();
      showToast(`已创建分组: ${name}`);
    } catch (e) {
      showToast("创建失败: " + e);
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
      showToast("已重命名");
    } catch (e) {
      showToast("重命名失败: " + e);
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
      showToast("已删除");
    } catch (e) {
      showToast("删除失败: " + e);
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
      showToast("已保存");
    } catch (e) {
      showToast("保存失败: " + e);
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
      showToast("读取版本信息失败: " + e);
    }
  };

  // 设置即时生效：每次修改直接持久化，无需点保存
  const handleSaveSettings = async (s: AppSettings) => {
    setSettings(s);
    try {
      await saveSettings(s);
    } catch (e) {
      showToast("保存设置失败: " + e);
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
      showToast("已移动");
    } catch (e) {
      showToast("移动失败: " + e);
    }
  };

  const baseUrl = rootInfo.baseUrl || "";

  // ---------- 渲染 ----------
  if (!workspace) {
    return (
      <div className="app">
        <div className="toolbar">
          <div className="logo">
            <div className="logo-badge">API</div>
            <span>API Manager</span>
          </div>
          <div className="toolbar-spacer" />
          <span style={{ color: "var(--text-faint)", fontSize: 12 }}>
            API 文档 · 测试 · Mock · v{version}
          </span>
        </div>
        <div className="landing">
          <div className="landing-logo">📂</div>
          <h1>API Manager</h1>
          <p>接口文档 · 接口测试 · Mock 服务，一站式管理</p>
          <button className="btn primary" style={{ fontSize: 14, padding: "10px 24px" }} onClick={handlePickWorkspace}>
            选择工作目录
          </button>
          <div className="file-tree-note">
            目录结构约定：
            <br />├── __info.json &nbsp;// 根目录描述（name / description / baseUrl / mockPort）
            <br />├── 分组/
            <br />│&nbsp;&nbsp;├── __info.json &nbsp;// 分组描述
            <br />│&nbsp;&nbsp;└── 接口.json &nbsp;// 一个接口一个 JSON 文件
          </div>
          <div className="hint">选择一个已按约定组织的目录，或直接选择空目录从零开始</div>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <div className="toolbar">
        <div className="logo">
          <div className="logo-badge">API</div>
          <span>API Manager</span>
          {version && <span className="logo-version">v{version}</span>}
        </div>
        <div className="workspace-chip" title="点击更换工作目录" onClick={handlePickWorkspace}>
          📁 {workspace}
        </div>
        <div className="toolbar-spacer" />
        <div className="env-box">
          <span style={{ fontSize: 12, color: "var(--text-dim)" }}>环境</span>
          <select
            className="env-select"
            value={envs.active || ""}
            onChange={(e) => handleEnvSwitch(e.target.value)}
            title="全局环境变量（请求时 {{变量名}} 会被替换）"
          >
            <option value="">（无环境）</option>
            {envs.environments.map((e) => (
              <option key={e.name} value={e.name}>
                {e.name}
              </option>
            ))}
          </select>
          <button
            className="btn"
            disabled={!activeEnv}
            title={activeEnv ? `查看 / 管理当前环境「${activeEnv.name}」的变量值` : "请先在工具栏选择环境"}
            onClick={() => setEnvValue(true)}
          >
            📋
          </button>
          <button className="btn" title="管理环境变量" onClick={() => setEnvModal(true)}>
            🌐
          </button>
        </div>
        {settings.enableMock && (
          <div className="mock-box">
            <span style={{ fontSize: 12, color: "var(--text-dim)" }}>Mock · {settings.mockPort}</span>
            <button className={`switch ${mock.running ? "on" : ""}`} onClick={toggleMock} title={`启动/停止 Mock 服务（端口 ${settings.mockPort}，可在设置中修改）`} />
            <span className="mock-status">
              {mock.running ? `运行中 ${mock.routeCount} 条路由` : "未运行"}
            </span>
          </div>
        )}
        <button className="btn" onClick={async () => { await reloadTree(); showToast("已刷新"); }}>
          🔄
        </button>
      </div>

      <div className="main">
        <Sidebar
          width={sidebarWidth}
          tree={tree}
          selectedPath={selectedPath}
          view={view}
          onSwitchView={switchView}
          onSelect={selectNode}
          onNewApi={(parent) => openModal("newApi", parent)}
          onNewFolder={(parent) => openModal("newFolder", parent)}
          onRename={(node) => openModal("rename", "", node)}
          onDelete={(node) => openModal("delete", "", node)}
          onEditInfo={(node) => openInfoModal(node)}
          onVersions={openVersions}
          onStats={setStatsNode}
          onOpenSettings={() => setSettingsOpen(true)}
          onImportPostman={() => void handleImportPostman()}
          vcs={vcs && settings.syncRemote ? vcs : null}
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
        />
        <div
          className="resizer"
          onMouseDown={startResize}
          onDoubleClick={() => {
            setSidebarWidth(310);
            sidebarWidthRef.current = 310;
            localStorage.setItem("sidebar-width", "310");
          }}
          title="拖动调整侧边栏宽度，双击还原"
        />

        <div
          className="content"
          onContextMenu={(e) => {
            // 右侧区域禁止右键（输入框/文本域保留原生菜单以便粘贴）
            const t = e.target as HTMLElement;
            if (t.tagName === "INPUT" || t.tagName === "TEXTAREA") return;
            e.preventDefault();
          }}
        >
          {view === "history" ? (
            <div className="history-view-content">
              <HistoryDetail detail={history.detail} loading={history.detailLoading} />
            </div>
          ) : api ? (
            <>
              <Editor
                style={{ height: `${editorRatio * 100}%` }}
                api={api}
                baseUrl={baseUrl}
                onChange={(a) => {
                  setApi(a);
                  setDirty(true);
                }}
                onSend={handleSend}
                onSaveVersion={handleSaveVersion}
                enableVersion={settings.enableVersion}
                sending={sending}
                onCommit={handleAutoSave}
              />
              <div
                className="v-resizer"
                onMouseDown={startVResize}
                onDoubleClick={() => {
                  setEditorRatio(0.45);
                  editorRatioRef.current = 0.45;
                  localStorage.setItem("editor-ratio", "0.45");
                }}
                title="拖动调整编辑区 / 响应区高度，双击还原"
              />
              <Response result={response} sending={sending} />
            </>
          ) : (
            <div
              className="empty-editor"
              onContextMenu={(e) => {
                e.preventDefault();
                setEmptyMenu({
                  x: Math.min(e.clientX, window.innerWidth - 190),
                  y: Math.min(e.clientY, window.innerHeight - 160),
                });
              }}
            >
              <span className="big">📄</span>
              <span>从左侧选择一个接口开始（右键可新建接口 / 分组）</span>
            </div>
          )}
        </div>
      </div>

      {toast && <div className="toast">{toast}</div>}

      {notify && (
        <div className="notify-pop" role="alert">
          <div className="notify-pop-head">
            <span className="notify-pop-title">⚠️ {notify.title}</span>
            <button
              className="notify-pop-close"
              onClick={() => setNotify(null)}
              title="关闭"
              aria-label="关闭"
            >
              ✕
            </button>
          </div>
          <pre className="notify-pop-body">{notify.body}</pre>
        </div>
      )}

      {emptyMenu && (
        <div className="node-ctx-menu" style={{ left: emptyMenu.x, top: emptyMenu.y }}>
          <button
            onClick={() => {
              openModal("newApi", "");
              setEmptyMenu(null);
            }}
          >
            🌐 新增接口
          </button>
          <button
            onClick={() => {
              openModal("newFolder", "");
              setEmptyMenu(null);
            }}
          >
            📁 新增分组
          </button>
        </div>
      )}

      {versionModal && (
        <VersionModal
          api={versionModal.api}
          versions={versionModal.versions}
          onClose={() => setVersionModal(null)}
        />
      )}

      {statsNode && <StatsModal node={statsNode} onClose={() => setStatsNode(null)} />}

      {settingsOpen && (
        <SettingsModal
          settings={settings}
          appVersion={version}
          vcs={vcs}
          onClose={() => setSettingsOpen(false)}
          onSave={handleSaveSettings}
        />
      )}

      {envModal && (
        <EnvModal
          envs={envs}
          onClose={() => setEnvModal(false)}
          onSave={handleSaveEnv}
        />
      )}

      {envValue && activeEnv && (
        <EnvValueModal
          name={activeEnv.name}
          variables={activeEnv.variables}
          onSave={handleSaveEnvValues}
          onClose={() => setEnvValue(false)}
          maskClassName="modal-mask-top"
        />
      )}

      {modal?.type === "newApi" && (
        <Modal
          title="新建接口"
          onClose={() => setModal(null)}
          footer={
            <>
              <button className="btn" onClick={() => setModal(null)}>取消</button>
              <button className="btn primary" onClick={doNewApi}>创建</button>
            </>
          }
        >
          <label>
            接口名称
            <input
              autoFocus
              value={modalText}
              onChange={(e) => setModalText(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && doNewApi()}
              placeholder="例如：获取用户信息"
            />
          </label>
          <label>
            保存位置
            <input value={modal.parent || workspace!} disabled style={{ opacity: 0.6 }} />
          </label>
        </Modal>
      )}

      {modal?.type === "newFolder" && (
        <Modal
          title="新建分组"
          onClose={() => setModal(null)}
          footer={
            <>
              <button className="btn" onClick={() => setModal(null)}>取消</button>
              <button className="btn primary" onClick={doNewFolder}>创建</button>
            </>
          }
        >
          <label>
            分组名称
            <input
              autoFocus
              value={modalText}
              onChange={(e) => setModalText(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && doNewFolder()}
              placeholder="例如：用户管理"
            />
          </label>
          <label>
            保存位置
            <input value={modal.parent || workspace!} disabled style={{ opacity: 0.6 }} />
          </label>
        </Modal>
      )}

      {modal?.type === "rename" && modal.target && (
        <Modal
          title="重命名"
          onClose={() => setModal(null)}
          footer={
            <>
              <button className="btn" onClick={() => setModal(null)}>取消</button>
              <button className="btn primary" onClick={doRename}>确定</button>
            </>
          }
        >
          <label>
            新名称
            <input
              autoFocus
              value={modalText}
              onChange={(e) => setModalText(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && doRename()}
            />
          </label>
        </Modal>
      )}

      {modal?.type === "delete" && modal.target && (
        <Modal
          title="确认删除"
          onClose={() => setModal(null)}
          footer={
            <>
              <button className="btn" onClick={() => setModal(null)}>取消</button>
              <button className="btn danger" onClick={doDelete}>删除</button>
            </>
          }
        >
          <div style={{ fontSize: 13, color: "var(--text-dim)" }}>
            确定要删除 <b style={{ color: "var(--text)" }}>{modal.target.name}</b> 吗？
            {modal.target.kind === "folder" && <div style={{ marginTop: 6 }}>将连同其下所有接口一并删除，此操作不可恢复！</div>}
          </div>
        </Modal>
      )}
      {modal?.type === "demo" && (
        <Modal
          title="生成演示案例"
          onClose={() => void closeDemoModal(false)}
          footer={
            <>
              <button className="btn" onClick={() => void closeDemoModal(false)}>
                不生成
              </button>
              <button className="btn primary" onClick={() => void closeDemoModal(demoCreate)}>
                确定
              </button>
            </>
          }
        >
          <div style={{ fontSize: 13, color: "var(--text-dim)", lineHeight: 1.7 }}>
            检测到这是一个新的工作目录（根目录没有 __info.json）。可以自动生成几个演示接口和环境变量，方便快速体验。
          </div>
          <label className="demo-check">
            <input
              type="checkbox"
              checked={demoCreate}
              onChange={(e) => setDemoCreate(e.target.checked)}
            />
            生成演示案例（用户管理 / 订单管理 分组 + 开发 / 生产环境）
          </label>
        </Modal>
      )}
      {modal?.type === "info" && modal.target && (
        <Modal
          title={`分组信息 - ${modal.target.name}`}
          onClose={() => setModal(null)}
          footer={
            <>
              <button className="btn" onClick={() => setModal(null)}>取消</button>
              <button className="btn primary" onClick={doSaveInfo}>保存</button>
            </>
          }
        >
          <label>
            名称
            <input
              autoFocus
              value={infoForm.name}
              onChange={(e) => setInfoForm({ ...infoForm, name: e.target.value })}
              placeholder="显示名称"
            />
          </label>
          <label>
            描述
            <textarea
              value={infoForm.description}
              onChange={(e) => setInfoForm({ ...infoForm, description: e.target.value })}
              placeholder="描述该分组的用途"
            />
          </label>
        </Modal>
      )}
    </div>
  );
}
