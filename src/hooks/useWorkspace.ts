import { useCallback, useEffect, useRef, useState } from "react";
import {
  getCurrentVersion,
  readApi,
  readEnv,
  readInfo,
  readTree,
  setWorkspaceSelectedApi,
  vcsInfo,
} from "../commands";
import { ApiFile, InfoJson, TreeNode } from "../types";

/** 转义正则特殊字符（用于按字面量构造 {变量名} 匹配） */
export const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** 计算路径相对工作区根的 '/'-分隔路径（统一分隔符）；路径不在工作区内或本身就是根时返回 null */
export function relOfPath(p: string, ws: string): string | null {
  const norm = (s: string) => s.replace(/\\/g, "/").replace(/\/+$/, "");
  const base = norm(ws);
  const full = norm(p);
  if (full === base) return null;
  if (base === "/") return full.replace(/^\/+/, "") || null;
  const prefix = base.endsWith("/") ? base : `${base}/`;
  return full.startsWith(prefix) ? full.slice(prefix.length) || null : null;
}

/** 按相对路径在工作区树中找节点（找不到返回 null） */
function findNodeByRel(node: TreeNode, ws: string, rel: string): TreeNode | null {
  if (relOfPath(node.path, ws) === rel) return node;
  for (const c of node.children || []) {
    const r = findNodeByRel(c, ws, rel);
    if (r) return r;
  }
  return null;
}

/** 按绝对路径在工作区树中找节点 */
function findNodeByPath(node: TreeNode, path: string): TreeNode | null {
  if (node.path === path) return node;
  for (const c of node.children || []) {
    const r = findNodeByPath(c, path);
    if (r) return r;
  }
  return null;
}

/**
 * 工作区：当前工作目录、树、选中接口、版本号。
 * 打开/刷新/选中接口的核心逻辑在这里（打开工作目录的询问流程由 App 组合）。
 */
export function useWorkspace(opts: {
  onEnvHydrate: (env: Awaited<ReturnType<typeof readEnv>>) => void;
  onVcs: (vcs: "git" | "svn" | null) => void;
}) {
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [treeLoading, setTreeLoading] = useState(false);
  const [rootInfo, setRootInfo] = useState<InfoJson>({});
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [api, setApi] = useState<ApiFile | null>(null);
  const [dirty, setDirty] = useState(false);
  const [currentVersion, setCurrentVersion] = useState(0);

  const { onEnvHydrate, onVcs } = opts;

  /** 查询接口当前版本号（保存按钮 tip 展示用） */
  const refreshVersion = useCallback(async (uuid: string) => {
    try {
      setCurrentVersion(await getCurrentVersion(uuid));
    } catch {
      setCurrentVersion(0);
    }
  }, []);

  function findFirstApi(node: TreeNode): TreeNode | null {
    if (node.kind === "api") return node;
    for (const c of node.children || []) {
      const r = findFirstApi(c);
      if (r) return r;
    }
    return null;
  }

  const loadAll = useCallback(
    async (ws: string) => {
      setTreeLoading(true);
      try {
        const [t2, info, e] = await Promise.all([readTree(), readInfo(ws), readEnv()]);
        setTree(t2);
        setRootInfo(info || {});
        onEnvHydrate(e);
        // 检测工作目录版本控制（.git / .svn）
        vcsInfo()
          .then((r) => onVcs(r.vcs))
          .catch(() => onVcs(null));
        // 自动选中接口：优先重开前最后使用的接口（记录在根 __info.json selectedApi），
        // 找不到（记录缺失 / 文件被删除 / 移动）则退回选中第一个接口
        let target: TreeNode | null = null;
        if (info?.selectedApi) target = findNodeByRel(t2, ws, info.selectedApi);
        if (!target || target.kind !== "api") target = findFirstApi(t2);
        if (target) {
          setSelectedPath(target.path);
          const data = await readApi(target.path);
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
    },
    [onEnvHydrate, onVcs, refreshVersion]
  );

  const reloadTree = useCallback(async (showLoading = false) => {
    if (showLoading) setTreeLoading(true);
    try {
      const t2 = await readTree();
      setTree(t2);
    } finally {
      if (showLoading) setTreeLoading(false);
    }
  }, []);

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
    setTree((t2) => (t2 ? patchNodeMethod(t2, selectedPath, api.method) : t2));
  }, [selectedPath, api?.method]);

  // Mock 开关变化时同步刷新左侧列表的 Mock 圆点
  useEffect(() => {
    if (!api || !selectedPath) return;
    setTree((t2) => (t2 ? patchNodeMock(t2, selectedPath, !!api.mock?.enabled) : t2));
  }, [selectedPath, api?.mock?.enabled]);

  /** 选中接口：当前接口有改动先保存，再加载新接口 */
  const selectNode = useCallback(
    async (
      node: TreeNode,
      currentApi: ApiFile | null,
      currentDirty: boolean,
      saveCurrent: () => Promise<void>
    ) => {
      if (currentDirty && currentApi) {
        try {
          await saveCurrent();
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
      void refreshVersion(data.uuid);
    },
    [refreshVersion]
  );

  // 最近选中的接口变化时记录到根 __info.json（selectedApi，相对根目录路径），
  // 重开工作区时默认选中它。只记录接口节点（分组/清空不写）；同一工作区值未变不重复写。
  const lastSavedRel = useRef<{ ws: string; rel: string } | null>(null);
  useEffect(() => {
    if (!workspace || !tree || !selectedPath) return;
    const node = findNodeByPath(tree, selectedPath);
    if (!node || node.kind !== "api") return;
    const rel = relOfPath(selectedPath, workspace);
    if (rel == null) return;
    if (lastSavedRel.current?.ws === workspace && lastSavedRel.current.rel === rel) return;
    lastSavedRel.current = { ws: workspace, rel };
    void setWorkspaceSelectedApi(rel).catch(() => {});
  }, [workspace, tree, selectedPath]);

  return {
    workspace,
    setWorkspace,
    tree,
    setTree,
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
    selectNode,
  };
}
