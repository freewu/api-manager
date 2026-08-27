import { Suspense, lazy, useCallback, useEffect, useState } from "react";
import {
  createDemo,
  getWorkspace,
  hasWorkspaceInfo,
  listVersions,
  moveEntry,
  openWorkspace,
  pickWorkspace,
  readApi,
  readApiVersion,
  readEnv,
  saveApi,
  saveApiVersion,
  saveInfo,
  reorderChildren,
  toggleDeprecated,
} from "./commands";
import { ObjectDef, TreeNode } from "./types";
import { AppModals } from "./components/AppModals";
import { CurlImportModal } from "./components/CurlImportModal";
import ImportResultModal from "./components/ImportResultModal";
import { AppToolbar } from "./components/AppToolbar";
import { RightPane } from "./components/RightPane";
import { AppView, Sidebar } from "./components/Sidebar";
import { useGenLogs } from "./hooks/useGenLogs";
import { useHistory } from "./hooks/useHistory";
import { useUi } from "./hooks/useUi";
import { useSettings } from "./hooks/useSettings";
import { useBootstrap } from "./hooks/useBootstrap";
import { useWorkspace } from "./hooks/useWorkspace";
import { useRequests } from "./hooks/useRequests";
import { useEnvs } from "./hooks/useEnvs";
import { useMock } from "./hooks/useMock";
import { useModals } from "./hooks/useModals";
import { useImports } from "./hooks/useImports";
import { useVcs } from "./hooks/useVcs";
import { useObjects } from "./hooks/useObjects";
import GenDataModal from "./components/GenDataModal";
import { GenLogItem } from "./commands";
import { useT } from "./i18n";

/** 切换工作目录转场动画中的青蛙图标（SVG 内联，随代码打包，无需外部资源） */
const FROG_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120">
  <ellipse cx="60" cy="80" rx="42" ry="34" fill="#4caf50"/>
  <circle cx="38" cy="42" r="20" fill="#66bb6a"/>
  <circle cx="82" cy="42" r="20" fill="#66bb6a"/>
  <circle cx="38" cy="42" r="12" fill="#ffffff"/>
  <circle cx="82" cy="42" r="12" fill="#ffffff"/>
  <circle cx="41" cy="44" r="5.5" fill="#1b5e20"/>
  <circle cx="79" cy="44" r="5.5" fill="#1b5e20"/>
  <circle cx="43" cy="42" r="2" fill="#ffffff"/>
  <circle cx="81" cy="42" r="2" fill="#ffffff"/>
  <circle cx="55" cy="68" r="1.8" fill="#2e7d32"/>
  <circle cx="65" cy="68" r="1.8" fill="#2e7d32"/>
  <path d="M44 74 Q60 92 76 74" stroke="#2e7d32" stroke-width="3.5" fill="none" stroke-linecap="round"/>
</svg>`;

// 非首屏组件懒加载：仅在需要时下载对应 chunk
const Landing = lazy(() => import("./components/Landing").then((m) => ({ default: m.Landing })));
const ApiDocModal = lazy(() =>
  import("./components/ApiDocModal").then((m) => ({ default: m.ApiDocModal }))
);

export default function App() {
  const t = useT();

  // ---------- 界面状态（toast / 分栏 / 空菜单） ----------
  const ui = useUi();
  const { toast, showToast, sidebarWidth, startResize, resetSidebarWidth, editorRatio, startVResize, resetEditorRatio, emptyMenu, setEmptyMenu } = ui;

  // ---------- 数据生成记录（视图模式，与请求历史一致） ----------
  const genLogs = useGenLogs();
  /** 记录详情「重新生成」：预填配置打开数据生成弹窗 */
  const [genRegen, setGenRegen] = useState<{ obj: ObjectDef; rec: GenLogItem } | null>(null);

  // ---------- 设置 ----------
  const settingsHook = useSettings((e) => showToast(t("toast.saveSettingsFailed", { err: e })));
  const { settings, setSettings: saveSettings, recentLimit } = settingsHook;

  // ---------- 历史 ----------
  const history = useHistory();

  // ---------- Mock ----------
  const mockHook = useMock({ mockPort: settings.mockPort, onToast: showToast, t });
  const { mock, setMock, reloadMockIfRunning, toggleMock } = mockHook;

  // ---------- 环境变量 ----------
  const envHook = useEnvs({
    mockRunning: mock.running,
    onMockReloaded: (s) => setMock(s),
    onToast: showToast,
    t,
  });
  const { envs, envModal, setEnvModal, envValue, setEnvValue, activeEnv, switchActive: handleEnvSwitch, save: handleSaveEnv, saveValues: handleSaveEnvValues, hydrate: hydrateEnvs } = envHook;

  // ---------- 启动（版本/托盘事件/主题） ----------
  const boot = useBootstrap({
    displayMode: settings.displayMode,
    onLanguageChanged: (lang) => settingsHook.setSettingsRaw({ ...settings, language: lang }),
    onUpdateAvailable: () => {},
    onOpenEnvEditor: () => setEnvModal(true),
    onMockChanged: (s) => setMock(s),
  });
  const { version, recent, setRecent, updateInfo, showUpdateModal, setShowUpdateModal } = boot;

  // ---------- 工作区 ----------
  const ws = useWorkspace({
    onEnvHydrate: hydrateEnvs,
    onVcs: (v) => vcsHook.setVcs(v),
  });
  const {
    workspace,
    setWorkspace,
    tree,
    treeLoading,
    rootInfo,
    setRootInfo,
    selectedPath,
    setSelectedPath,
    api,
    setApi,
    dirty,
    setDirty,
    currentVersion,
    refreshVersion,
    loadAll,
    reloadTree,
    selectNode: wsSelectNode,
  } = ws;

  // ---------- 对象管理 ----------
  const objects = useObjects(workspace);
  const {
    store: objectsStore,
    usage: objectsUsage,
    save: saveObjectsStore,
    doImport: importObjectsJson,
    doImportDdl: importObjectsDdl,
  } = objects;
  /** 生成记录「重新生成」：按记录配置打开数据生成弹窗 */
  const handleGenLogsRegen = useCallback(
    (rec: GenLogItem) => {
      const obj = objectsStore.objects.find((o) => o.uuid === rec.object_uuid);
      if (!obj) {
        showToast(t("objects.genDataObjMissing"));
        return;
      }
      setGenRegen({ obj, rec });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [objectsStore, t]
  );
  // 对象管理中当前选中对象（按 uuid 定位，左侧树点击 → 右侧展开配置）
  const [objectsSelectedUuid, setObjectsSelectedUuid] = useState<string | null>(null);
  // 右侧空状态「新增 / 导入」按钮请求信号（每次 +1 触发左侧弹窗）
  const [objectsReq, setObjectsReq] = useState({ new: 0, imp: 0 });

  // ---------- 弹窗操作 ----------
  const modals = useModals({
    workspace,
    selectedPath,
    settings,
    reloadTree,
    reloadMockIfRunning,
    onApiReplaced: (data, path) => {
      setSelectedPath(path);
      setApi(data);
      setDirty(false);
      setResponse(null);
      void refreshVersion(data.uuid);
    },
    onApiCleared: () => {
      setSelectedPath(null);
      setApi(null);
      setResponse(null);
    },
    onToast: showToast,
    t,
  });
  const {
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
  } = modals;

  // ---------- 请求（HTTP / WebSocket） ----------
  const baseUrl = rootInfo.baseUrl || "";
  const req = useRequests({ api, envs, baseUrl, onToast: showToast, onEnvChanged: () => void readEnv().then(hydrateEnvs), t });
  const {
    response,
    setResponse,
    exampleVersion,
    sending,
    hideResponse,
    setHideResponse,
    wsConnected,
    wsConnecting,
    wsEntries,
    handleSend,
    handleSaveExample,
    closeWsConnection,
  } = req;

  // ---------- 导入 ----------
  const imports = useImports({
    workspace,
    loadAll,
    mockRunning: mock.running,
    reloadMockIfRunning,
    onToast: showToast,
    t,
  });

  // ---------- 版本控制 ----------
  const vcsHook = useVcs({
    syncRemote: settings.syncRemote,
    onToast: showToast,
    onNotify: (n) => setNotify(n),
    t,
  });
  const { handleVcsSync, handleVcsCommitPush } = vcsHook;

  // ---------- 视图切换 ----------
  const [view, setView] = useState<AppView>("api");
  /** 切换工作目录时的转场动画状态（遮罩淡入 → 切换 → 遮罩淡出） */
  const [switching, setSwitching] = useState(false);
  /** 切换遮罩淡出阶段 */
  const [switchOut, setSwitchOut] = useState(false);

  // 进入数据生成记录页面时刷新列表
  useEffect(() => {
    if (view === "genlogs") void genLogs.reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view]);
  const switchView = (v: AppView) => {
    setView(v);
    // 每次进入历史视图都自动刷新一次列表
    if (v === "history") history.reload();
  };

  // ---------- 启动流程：加载设置（界面语言等，与托盘语言保持一致） ----------
  useEffect(() => {
    void settingsHook.load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ---------- 启动流程：恢复上次工作区 ----------
  useEffect(() => {
    (async () => {
      const w = await getWorkspace();
      if (w) {
        setWorkspace(w);
        if (!(await hasWorkspaceInfo())) {
          // 新的工作目录（没有 __info.json）：询问是否生成演示案例
          setDemoCreate(true);
          setModal({ type: "demo", parent: w });
        } else {
          await loadAll(w);
        }
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ---------- 打开工作目录 ----------
  const finishOpenWorkspace = useCallback(
    async (w: string) => {
      // 切换动画：全屏遮罩淡入 → 回到接口管理视图并加载数据 → 遮罩淡出
      setSwitching(true);
      setSwitchOut(false);
      setView("api");
      await new Promise((r) => setTimeout(r, 260)); // 等待遮罩淡入
      setWorkspace(w);
      setResponse(null);
      if (!(await hasWorkspaceInfo())) {
        // 新的工作目录（没有 __info.json）：询问是否生成演示案例
        setDemoCreate(true);
        setModal({ type: "demo", parent: w });
      } else {
        await loadAll(w);
        showToast(t("toast.opened", { ws: w }));
      }
      setSwitchOut(true);
      await new Promise((r) => setTimeout(r, 260)); // 等待遮罩淡出
      setSwitching(false);
    },
    [loadAll, showToast, t]
  );

  const handlePickWorkspace = useCallback(async () => {
    try {
      const w = await pickWorkspace();
      if (!w) return;
      setRecent((r) => [w, ...r.filter((x) => x !== w)]);
      await finishOpenWorkspace(w);
    } catch (e) {
      showToast(t("toast.openFailed", { err: String(e) }));
    }
  }, [finishOpenWorkspace, showToast, t]);

  const handleOpenRecent = useCallback(
    async (w: string) => {
      try {
        await openWorkspace(w);
        setRecent((r) => [w, ...r.filter((x) => x !== w)]);
        await finishOpenWorkspace(w);
      } catch (e) {
        showToast(t("toast.openFailed", { err: String(e) }));
      }
    },
    [finishOpenWorkspace, showToast, t]
  );

  /** 新工作目录（无 __info.json）询问后的收尾：按参数生成演示案例并加载 */
  const closeDemoModal = useCallback(
    async (create: boolean) => {
      const w = modal?.parent || workspace;
      setModal(null);
      if (w) {
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
            await saveInfo(w, {
              name: t("app.defaultWsName"),
              description: "",
              baseUrl: "",
              mockPort: 5050,
            });
          } catch {
            /* noop */
          }
        }
        await loadAll(w);
        if (create) {
          // 对象示例由后端直接写入 .object/，这里刷新对象列表，否则对象页看不到 demo 对象
          try {
            await objects.refresh();
          } catch {
            /* noop */
          }
        }
        if (!create) showToast(t("toast.opened", { ws: workspace ?? modal?.parent ?? "" }));
      }
    },
    [modal?.parent, workspace, loadAll, objects.refresh, showToast, t]
  );

  // ---------- 接口编辑 ----------
  const handleAutoSave = useCallback(async () => {
    if (!dirty || !api || !selectedPath) return;
    try {
      await saveApi(selectedPath, api);
      setDirty(false);
      showToast(t("toast.saved"));
      // Mock 服务运行中：热重载路由，使 mock tab 的最新配置（enabled/body 等）立即生效
      await reloadMockIfRunning(mock.running);
    } catch (e) {
      showToast(t("toast.saveFailed", { err: String(e) }));
    }
  }, [dirty, api, selectedPath, showToast, t, reloadMockIfRunning, mock.running]);

  // 选中接口：切换前自动保存当前接口改动，并清空响应
  const selectNode = useCallback(
    async (node: TreeNode) => {
      setResponse(null);
      await wsSelectNode(node, api, dirty, handleAutoSave);
    },
    [wsSelectNode, api, selectedPath, dirty, handleAutoSave]
  );

  // 对象管理：跳转到引用该对象的接口
  const jumpToApi = useCallback(
    (path: string) => {
      const findNode = (n: TreeNode | null, p: string): TreeNode | null => {
        if (!n) return null;
        if (n.path === p) return n;
        for (const c of n.children || []) {
          const r = findNode(c, p);
          if (r) return r;
        }
        return null;
      };
      const node = findNode(tree, path);
      if (node) {
        setView("api");
        void selectNode(node);
      } else {
        showToast(t("app.nodeNotFound"));
      }
    },
    [tree, selectNode, t]
  );

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

  // 拖动排序：同级接口 / 分组整体重排（把有序子项列表写入父分组 __info.json）
  const handleReorder = async (parent: string, paths: string[]) => {
    try {
      await reorderChildren(parent, paths);
      await reloadTree();
      showToast(t("toast.reordered"));
    } catch (e) {
      showToast(t("toast.reorderFailed", { err: String(e) }));
    }
  };

  // 标记 / 取消标记"已废弃"（接口或分组），成功后在左侧树与当前编辑的接口上即时生效
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

  // ---------- 渲染 ----------
  return (
    <Suspense fallback={<div className="app-chunk-loading" />}>
      {!workspace ? (
        <Landing
          version={version}
          updateInfo={updateInfo}
          recent={recent}
          recentLimit={recentLimit}
          onPickWorkspace={() => void handlePickWorkspace()}
          onOpenRecent={(w) => void handleOpenRecent(w)}
          onOpenUpdate={() => setShowUpdateModal(true)}
        />
      ) : (
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
            onOpenRecent={(w) => void handleOpenRecent(w)}
            onSwitchEnv={(name) => void handleEnvSwitch(name)}
            onOpenEnvValue={() => setEnvValue(true)}
            onOpenEnvModal={() => setEnvModal(true)}
            onToggleMock={async () => {
              // 启动/停止前先保存当前接口改动，确保 mock 服务按 mock tab 最新配置返回
              await handleAutoSave();
              await toggleMock();
            }}
            onRefresh={async () => {
              await reloadTree(true);
              showToast(t("toast.refreshed"));
            }}
            onOpenUpdate={() => setShowUpdateModal(true)}
            onToast={showToast}
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
              onOpenGenLogs={() => setView(view === "genlogs" ? "api" : "genlogs")}
              genLogsRecords={genLogs.records}
              genLogsLoading={genLogs.loading}
              genLogsSelected={genLogs.selectedId}
              onGenLogsSelect={genLogs.select}
              onGenLogsReload={genLogs.reload}
              onImportPostman={() => void imports.handleImportPostman()}
              onImportCurl={modals.openCurlImport}
              onImportOpenApi={() => void imports.handleImportOpenApi()}
              onImportMarkdown={() => void imports.handleImportMarkdown()}
              onImportApifox={() => void imports.handleImportApifox()}
              onImportApipost={() => void imports.handleImportApipost()}
              onImportRaml={() => void imports.handleImportRaml()}
              onImportWadl={() => void imports.handleImportWadl()}
              onImportHar={() => void imports.handleImportHar()}
              onImportYapi={() => void imports.handleImportYapi()}
              onImportEolink={() => void imports.handleImportEolink()}
              onImportInsomnia={() => void imports.handleImportInsomnia()}
              onImportJmeter={() => void imports.handleImportJmeter()}
              onImportApiDoc={() => void imports.handleImportApiDoc()}
              onImportExtra={(format) => void imports.handleImportExtra(format)}
              settings={settings}
              onViewMarkdown={(node) => void handleViewMarkdown(node)}
              onViewApiDoc={(node) => void handleViewApiDoc(node)}
              onExport={() => openExport()}
              onExportNode={(node) => openExport(node)}
              vcs={null} // 同步远程功能暂时隐藏（后端命令保留，恢复时改回 vcs && settings.syncRemote ? vcs : null）
              onVcsSync={() => void handleVcsSync()}
              onVcsCommitPush={() => void handleVcsCommitPush()}
              onMove={handleMove}
              onReorder={handleReorder}
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
              objectsStore={objectsStore}
              objectsUsage={objectsUsage}
              onObjectsSave={saveObjectsStore}
              onObjectsImport={importObjectsJson}
              onObjectsImportDdl={importObjectsDdl}
              onObjectsToast={showToast}
              objectsSelectedUuid={objectsSelectedUuid}
              onObjectsSelect={setObjectsSelectedUuid}
              objectsNewReq={objectsReq.new}
              objectsImportReq={objectsReq.imp}
            />
            <RightPane
              view={view}
              api={api}
              historyDetail={history.detail}
              historyDetailLoading={history.detailLoading}
              historyDiff={history.diffPair}
              historyDiffLoading={history.diffLoading}
              onHistoryDiffExit={history.exitDiff}
              genLogsDetail={genLogs.records.find((r) => r.file === genLogs.selectedId) ?? null}
              onGenLogsRegen={handleGenLogsRegen}
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
              onTabChange={(tab) =>
                setHideResponse(["response", "mock", "prescript", "desc", "doc", "code", "examples"].includes(tab))
              }
              onEnvChanged={() => void readEnv().then(hydrateEnvs)}
              onStartVResize={startVResize}
              onResetRatio={resetEditorRatio}
              onWsDisconnect={closeWsConnection}
              onResizeStart={startResize}
              onResizeReset={resetSidebarWidth}
              resizeTip={t("app.resizeSidebarTip")}
              onEmptyContextMenu={(e) => {
                e.preventDefault();
                setEmptyMenu({
                  x: Math.min(e.clientX, window.innerWidth - 190),
                  y: Math.min(e.clientY, window.innerHeight - 160),
                });
              }}
              objectsStore={objectsStore}
              onObjectsSave={saveObjectsStore}
              onObjectsImport={importObjectsJson}
              onObjectsImportDdl={importObjectsDdl}
              onObjectsJumpApi={jumpToApi}
              objectsSelectedUuid={objectsSelectedUuid}
              onObjectsSelect={setObjectsSelectedUuid}
              onObjectsRequestNew={() => setObjectsReq((r) => ({ ...r, new: r.new + 1 }))}
              onObjectsRequestImport={() => setObjectsReq((r) => ({ ...r, imp: r.imp + 1 }))}
              onObjectsToast={showToast}
              objectsList={objectsStore.objects}
            />
          </div>

          {genRegen && (
            <GenDataModal
              obj={genRegen.obj}
              initialProps={genRegen.rec.props.map((p) => ({ key: p.key, enabled: p.enabled, mock: p.mock }))}
              initialFormat={genRegen.rec.format}
              initialTable={genRegen.rec.table}
              initialCount={genRegen.rec.count}
              initialDir={genRegen.rec.dir}
              onClose={() => setGenRegen(null)}
              onDone={() => void genLogs.reload()}
              t={t}
            />
          )}

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
            onSaveSettings={saveSettings}
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

          <CurlImportModal
            open={modals.curlOpen}
            name={modals.curlName}
            onNameChange={modals.setCurlName}
            text={modals.curlText}
            onTextChange={modals.setCurlText}
            error={modals.curlError}
            onSave={() => void modals.doImportCurl()}
            onClose={() => modals.setCurlOpen(false)}
          />
          <ImportResultModal result={imports.importResult} onClose={imports.closeImportResult} />

          {apiDocView && (
            <ApiDocModal
              name={apiDocView.node.name}
              text={apiDocView.text}
              onClose={() => setApiDocView(null)}
            />
          )}
        </div>
      )}
      {switching && (
        <div className={`app-switch-overlay${switchOut ? " out" : ""}`}>
          <div
            className="app-switch-logo"
            aria-hidden="true"
            dangerouslySetInnerHTML={{ __html: FROG_SVG }}
          />
          <div className="app-switch-text">{t("app.switching")}</div>
        </div>
      )}
    </Suspense>
  );
}
