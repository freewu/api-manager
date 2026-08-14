import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  createApi,
  createFolder,
  deleteEntry,
  getAppVersion,
  getWorkspace,
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
  saveApi,
  saveEnv,
  saveInfo,
  sendRequest,
  updateTrayEnv,
} from "./commands";
import { Editor } from "./components/Editor";
import { EnvModal } from "./components/EnvModal";
import { EnvValueModal } from "./components/EnvValueModal";
import { Modal } from "./components/Modal";
import { Response } from "./components/Response";
import { Sidebar } from "./components/Sidebar";
import {
  ApiFile,
  EnvStore,
  EnvVariable,
  HttpRequestData,
  HttpResult,
  InfoJson,
  MockStatus,
  TreeNode,
  emptyEnv,
} from "./types";

interface ModalState {
  type: "newApi" | "newFolder" | "rename" | "delete" | "info";
  parent: string;
  target?: TreeNode;
}

interface InfoForm {
  name: string;
  description: string;
  baseUrl: string;
  mockPort: number;
}

const emptyInfoForm = (): InfoForm => ({
  name: "",
  description: "",
  baseUrl: "",
  mockPort: 5050,
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
  const [modal, setModal] = useState<ModalState | null>(null);
  const [modalText, setModalText] = useState("");
  const [infoForm, setInfoForm] = useState<InfoForm>(emptyInfoForm());
  const [toast, setToast] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [mockPort, setMockPort] = useState(5050);
  const [envs, setEnvs] = useState<EnvStore>(emptyEnv());
  const [envModal, setEnvModal] = useState(false);
  const [envValue, setEnvValue] = useState(false);
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
        await loadAll(ws);
      }
    })();
    mockStatus().then(setMock).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 托盘菜单点击「环境变量」-> 打开环境变量编辑器
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen("open-env-editor", () => setEnvModal(true));
    })();
    return () => unlisten?.();
  }, []);

  async function loadAll(ws: string) {
    const [t, info, e] = await Promise.all([readTree(), readInfo(ws), readEnv()]);
    setTree(t);
    setRootInfo(info || {});
    const envData = e || emptyEnv();
    setEnvs(envData);
    updateTrayEnv(envData.active || "").catch(() => {});
    if (info?.mockPort) setMockPort(info.mockPort);
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
      await loadAll(ws);
      showToast("已打开工作区");
    } catch (e) {
      showToast("打开失败: " + e);
    }
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
    } catch (e) {
      setResponse({ ok: false, status: 0, statusText: "", headers: [], body: "", timeMs: 0, size: 0, url: "", error: String(e) });
    } finally {
      setSending(false);
    }
  };

  const handleSave = async () => {
    if (!api || !selectedPath) return;
    try {
      await saveApi(selectedPath, api);
      setDirty(false);
      showToast("已保存");
      await reloadTree();
      if (mock.running) {
        const m = await mockReload();
        setMock(m);
      }
    } catch (e) {
      showToast("保存失败: " + e);
    }
  };

  const toggleMock = async () => {
    try {
      if (mock.running) {
        setMock(await mockStop());
        showToast("Mock 服务已停止");
      } else {
        const port = mockPort || 5050;
        setMock(await mockStart(port));
        showToast(`Mock 服务已启动: http://127.0.0.1:${port}`);
        setRootInfo({ ...rootInfo, mockPort: port });
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

  const openInfoModal = async (target?: TreeNode) => {
    const path = target ? target.path : workspace!;
    try {
      const info = await readInfo(path);
      setInfoForm({
        name: info.name || (target ? target.name : ""),
        description: info.description || "",
        baseUrl: info.baseUrl || "",
        mockPort: info.mockPort || 5050,
      });
      setModal({ type: "info", parent: path, target });
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
      const isRoot = !modal.target;
      await saveInfo(modal.parent, {
        name: infoForm.name.trim() || undefined,
        description: infoForm.description,
        ...(isRoot
          ? { baseUrl: infoForm.baseUrl.trim() || undefined, mockPort: infoForm.mockPort }
          : {}),
      });
      setModal(null);
      if (isRoot) {
        const info = await readInfo(workspace!);
        setRootInfo(info || {});
        if (info?.mockPort) setMockPort(info.mockPort);
      }
      await reloadTree();
      showToast("已保存");
    } catch (e) {
      showToast("保存失败: " + e);
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
        {api && (
          <button className="btn" onClick={handleSave} disabled={!dirty} title={dirty ? "保存当前接口" : "无修改"}>
            💾 保存{dirty ? " *" : ""}
          </button>
        )}
        <div className="mock-box">
          <span style={{ fontSize: 12, color: "var(--text-dim)" }}>Mock</span>
          <input
            className="mock-port-input"
            value={mockPort}
            onChange={(e) => setMockPort(Number(e.target.value.replace(/\D/g, "")) || 0)}
            disabled={mock.running}
            title="Mock 服务端口"
          />
          <button className={`switch ${mock.running ? "on" : ""}`} onClick={toggleMock} title="启动/停止 Mock 服务" />
          <span className="mock-status">
            {mock.running ? `运行中 ${mock.routeCount} 条路由` : "未运行"}
          </span>
        </div>
        <button className="btn" onClick={async () => { await reloadTree(); showToast("已刷新"); }}>
          🔄
        </button>
        <button className="btn" title="集合设置" onClick={() => openInfoModal()}>
          ⚙ 集合设置
        </button>
      </div>

      <div className="main">
        <Sidebar
          tree={tree}
          selectedPath={selectedPath}
          onSelect={selectNode}
          onNewApi={(parent) => openModal("newApi", parent)}
          onNewFolder={(parent) => openModal("newFolder", parent)}
          onRename={(node) => openModal("rename", "", node)}
          onDelete={(node) => openModal("delete", "", node)}
          onEditInfo={(node) => openInfoModal(node)}
        />

        <div className="content">
          {api ? (
            <>
              <Editor
                api={api}
                baseUrl={baseUrl}
                onChange={(a) => {
                  setApi(a);
                  setDirty(true);
                }}
                onSend={handleSend}
                sending={sending}
              />
              <Response result={response} sending={sending} />
            </>
          ) : (
            <div className="empty-editor">
              <span className="big">📄</span>
              <span>从左侧选择一个接口开始</span>
            </div>
          )}
        </div>
      </div>

      {toast && <div className="toast">{toast}</div>}

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
      {modal?.type === "info" && (
        <Modal
          title={modal.target ? `分组信息 - ${modal.target.name}` : "集合设置"}
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
              placeholder="描述该分组/集合的用途"
            />
          </label>
          {!modal.target && (
            <>
              <label>
                Base URL（接口请求前缀）
                <input
                  value={infoForm.baseUrl}
                  onChange={(e) => setInfoForm({ ...infoForm, baseUrl: e.target.value })}
                  placeholder="https://api.example.com"
                  spellCheck={false}
                />
              </label>
              <label>
                Mock 服务端口
                <input
                  type="number"
                  value={infoForm.mockPort}
                  onChange={(e) =>
                    setInfoForm({ ...infoForm, mockPort: Number(e.target.value) || 5050 })
                  }
                />
              </label>
            </>
          )}
        </Modal>
      )}
    </div>
  );
}
