import { useCallback, useEffect, useState } from "react";
import {
  getCurrentVersion,
  readApi,
  readEnv,
  readInfo,
  readTree,
  vcsInfo,
} from "../commands";
import { ApiFile, InfoJson, TreeNode } from "../types";

/** 转义正则特殊字符（用于按字面量构造 {变量名} 匹配） */
export const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

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
        // 自动选中第一个接口
        const first = findFirstApi(t2);
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
