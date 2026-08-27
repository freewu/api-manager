import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AppSettings, TreeNode } from "../types";
import { HistoryDay, HistorySummary, setFolderCollapsed } from "../commands";
import { HistoryList } from "./HistoryList";
import ObjectsTree from "./ObjectsTree";
import { ObjectImportResult, ObjectStore, ObjectUsageItem } from "../types";
import { FormatIcon } from "./FormatSelect";
import { useT } from "../i18n";
import { GenLogsList } from "./GenLogsList";
import { GenLogItem } from "../commands";
import iconHttp from "../assets/icon-http.png";
import iconWs from "../assets/icon-websocket.png";
import iconGql from "../assets/icon-graphql.png";
import iconSocketIo from "../assets/icon-socketio.png";

export type AppView = "api" | "history" | "objects" | "genlogs";

interface Props {
  width?: number;
  tree: TreeNode | null;
  /** 工作目录树加载中（显示加载动画） */
  loading?: boolean;
  selectedPath: string | null;
  view: AppView;
  onSwitchView: (v: AppView) => void;
  onSelect: (node: TreeNode) => void;
  onNewApi: (parent: string) => void;
  onNewFolder: (parent: string) => void;
  onRename: (node: TreeNode) => void;
  onCopy: (node: TreeNode) => void;
  onDelete: (node: TreeNode) => void;
  onToggleDeprecated: (node: TreeNode) => void;
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onStats?: (node: TreeNode) => void;
  onViewMarkdown?: (node: TreeNode) => void;
  onViewApiDoc?: (node: TreeNode) => void;
  onOpenSettings?: () => void;
  /** 打开数据生成记录管理 */
  onOpenGenLogs?: () => void;
  onImportPostman?: () => void;
  onImportCurl?: () => void;
  onImportOpenApi?: () => void;
  onImportMarkdown?: () => void;
  onImportApifox?: () => void;
  onImportApipost?: () => void;
  onImportRaml?: () => void;
  onImportWadl?: () => void;
  onImportHar?: () => void;
  onImportYapi?: () => void;
  onImportEolink?: () => void;
  onImportInsomnia?: () => void;
  onImportJmeter?: () => void;
  onImportApiDoc?: () => void;
  /** 扩展格式导入（apidog/bruno/apizza/nei/doclever/io-docs/easydoc/docway/hoppscotch/metersphere） */
  onImportExtra?: (format: string) => void;
  /** 当前设置（导入菜单按 importTypes 开关过滤格式） */
  settings?: AppSettings;
  onExport?: () => void;
  onExportNode?: (node: TreeNode) => void;
  /** 工作目录版本控制类型（.git / .svn），为空时不显示同步/提交按钮 */
  vcs?: "git" | "svn" | null;
  onVcsSync?: () => void;
  onVcsCommitPush?: () => void;
  onMove: (srcPath: string, dstDir: string) => Promise<void>;
  /** 拖动排序微调：把单个接口 / 分组的 order 改为指定值 */
  onReorderOne?: (path: string, order: number) => Promise<void>;
  enableVersion: boolean;
  // 请求历史列表数据（由 App 通过 useHistory 提供）
  historyRecords: HistorySummary[];
  historyDays: HistoryDay[];
  historyLoading: boolean;
  historyHasMore: boolean;
  historySelected: string | null;
  historyTotal: number;
  onHistorySelect: (id: string) => void;
  onHistoryLoadMore: () => void;
  onHistoryReload: () => void;
  onHistoryClear: () => void;
  // Diff 比对
  historyDiffMode: boolean;
  /** 数据生成记录（视图模式，与请求历史一致） */
  genLogsRecords: GenLogItem[];
  genLogsLoading: boolean;
  genLogsSelected: string | null;
  onGenLogsSelect: (id: string) => void;
  onGenLogsReload: () => void;
  historyDiffIds: string[];
  historyDiffError: string;
  onHistoryToggleDiffMode: (on: boolean) => void;
  onHistoryToggleDiffSelect: (r: HistorySummary) => void;
  onHistoryStartDiff: () => void;
  // 对象管理树
  objectsStore: ObjectStore;
  objectsUsage: ObjectUsageItem[];
  onObjectsSave: (store: ObjectStore) => Promise<ObjectStore>;
  objectsSelectedUuid: string | null;
  onObjectsSelect: (uuid: string | null) => void;
  /** 右侧空状态请求信号（每次 +1） */
  objectsNewReq: number;
  objectsImportReq: number;
  onObjectsImport: (name: string, group: string, json: string) => Promise<ObjectImportResult>;
  onObjectsImportDdl: (group: string, ddl: string) => Promise<ObjectImportResult>;
  onObjectsToast: (msg: string) => void;
}

interface CtxMenu {
  x: number;
  y: number;
  node: TreeNode;
}

function methodClass(method?: string) {
  return `method-${(method || "get").toLowerCase()}`;
}

// 取父目录（Windows 路径兼容）
function parentDir(p: string): string {
  const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return i > 0 ? p.slice(0, i) : "";
}

// 拖拽落点有效性：非自身、非自身子目录、非原地
// dstIsFolder：落点是目录时，src 已在该目录（原地）视为无效；落点是接口时允许同级排序
function validDrop(dragSrc: string, dst: string, dstIsFolder: boolean): boolean {
  if (!dst || dst === dragSrc) return false;
  if (dst.startsWith(dragSrc + "/") || dst.startsWith(dragSrc + "\\")) return false; // 子目录
  if (dstIsFolder && parentDir(dragSrc) === dst) return false; // 已在目标目录（原地）
  return true;
}
// 废弃状态筛选：all=全部 / active=未废弃 / deprecated=已废弃
type DepFilter = "all" | "active" | "deprecated";

/** 高级搜索可选的接口协议类型 */
const PROTOCOL_OPTIONS = [
  { id: "http", label: "HTTP" },
  { id: "websocket", label: "WebSocket" },
  { id: "socketio", label: "Socket.IO" },
  { id: "graphql", label: "GraphQL" },
] as const;

/** 高级搜索可选的接口 Method（WebSocket / GraphQL 接口无 Method） */
const METHOD_OPTIONS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

function NodeRow({
  node,
  depth,
  selectedPath,
  openMap,
  onToggleOpen,
  onSelect,
  onNewApi,
  onNewFolder,
  onRename,
  onCopy,
  onDelete,
  onToggleDeprecated,
  onEditInfo,
  onVersions,
  onStats,
  onViewMarkdown,
  onViewApiDoc,
  enableVersion,
  onContextMenu,
  filter,
  protocolFilters,
  methodFilters,
  depFilter,
  depInherited,
  dragSrc,
  dragOver,
  onDragStart,
  onDragEnd,
  onDragOverTarget,
  onDragLeaveTarget,
  onDropTarget,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  /** 文件夹展开状态（path → open），跨树刷新保留 */
  openMap: Record<string, boolean>;
  onToggleOpen: (path: string, open: boolean) => void;
  onSelect: (node: TreeNode) => void;
  onNewApi: (parent: string) => void;
  onNewFolder: (parent: string) => void;
  onRename: (node: TreeNode) => void;
  onCopy: (node: TreeNode) => void;
  onDelete: (node: TreeNode) => void;
  onToggleDeprecated: (node: TreeNode) => void;
  onEditInfo: (node: TreeNode) => void;
  onVersions: (node: TreeNode) => void;
  onStats?: (node: TreeNode) => void;
  onViewMarkdown?: (node: TreeNode) => void;
  onViewApiDoc?: (node: TreeNode) => void;
  enableVersion: boolean;
  tree: null;
  onContextMenu: (e: React.MouseEvent, node: TreeNode) => void;
  filter: string;
  /** 高级搜索：按接口协议类型多选过滤（空数组 = 不过滤） */
  protocolFilters: string[];
  /** 高级搜索：按接口 Method 多选过滤（空数组 = 不过滤） */
  methodFilters: string[];
  /** 废弃状态筛选选项 */
  depFilter: DepFilter;
  /** 父级分组是否已废弃（子节点继承渲染样式） */
  depInherited: boolean;
  dragSrc: string | null;
  dragOver: string | null;
  onDragStart: (path: string) => void;
  onDragEnd: () => void;
  onDragOverTarget: (dst: string) => void;
  onDragLeaveTarget: (dst: string) => void;
  onDropTarget: (
    src: string,
    dst: string,
    dstIsFolder: boolean,
    after: boolean,
    dstOrder?: number,
  ) => void;
}) {
  const t = useT();
  const [dropPos, setDropPos] = useState<"top" | "bottom" | null>(null);
  const isFolder = node.kind === "folder";
  // WebSocket 接口无 HTTP method
  const isWs = node.protocol === "websocket";
  // 展开状态提升到 Sidebar 顶层（openMap），导入/刷新重建树后保持不变
  const open = openMap[node.path] ?? node.collapsed !== true;
  // 已废弃：自身标记或继承自上层已废弃分组
  const deprecated = node.deprecated === true || depInherited;

  const protocolOk =
    isFolder ||
    protocolFilters.length === 0 ||
    protocolFilters.includes(node.protocol || "http");
  // WebSocket / GraphQL 接口无 Method：选中任何 Method 均不匹配
  const methodOk =
    isFolder ||
    methodFilters.length === 0 ||
    (isWs ? false : methodFilters.includes((node.method || "").toUpperCase()));

  const matches =
    protocolOk &&
    methodOk &&
    (!filter ||
      node.name.toLowerCase().includes(filter) ||
      (node.endpoint || "").toLowerCase().includes(filter));

  // 深度搜索：任意层级的后代命中关键词 / 命中协议+Method 过滤（导入的接口常嵌套在 导入分组→tag 分组 下）
  const childrenMatch = useMemo(() => {
    if (!isFolder || !node.children) return false;
    if (!filter && protocolFilters.length === 0 && methodFilters.length === 0) return false;
    const hit = (n: TreeNode): boolean => {
      if (n.kind === "folder") return !!n.children && n.children.some(hit);
      const pOk =
        protocolFilters.length === 0 || protocolFilters.includes(n.protocol || "http");
      const mOk =
        methodFilters.length === 0 ||
        (n.protocol === "websocket" || n.protocol === "socketio"
          ? false
          : methodFilters.includes((n.method || "").toUpperCase()));
      return (
        pOk &&
        mOk &&
        (n.name.toLowerCase().includes(filter) ||
          (n.endpoint || "").toLowerCase().includes(filter))
      );
    };
    return node.children.some(hit);
  }, [isFolder, node.children, filter, protocolFilters, methodFilters]);

  // 废弃状态筛选：自身命中 / 后代命中（递归，废弃分组视同其下内容均废弃）
  const depSelf =
    depFilter === "all" || (depFilter === "deprecated" ? deprecated : !deprecated);
  const depChildrenMatch = useMemo(() => {
    if (!isFolder || !node.children) return false;
    if (depFilter === "all") return false;
    const hit = (n: TreeNode, inherited: boolean): boolean => {
      const ed = n.deprecated === true || inherited;
      if (depFilter === "deprecated" ? ed : !ed) return true;
      if (n.kind === "folder" && n.children) return n.children.some((c) => hit(c, ed));
      return false;
    };
    return node.children.some((c) => hit(c, deprecated));
  }, [isFolder, node.children, depFilter, deprecated]);

  // 查询过滤生效时（关键词 / 协议 / Method），隐藏没有命中后代的分组（空分组）
  const filtering = !!filter || protocolFilters.length > 0 || methodFilters.length > 0;
  const visible =
    (isFolder && filtering ? childrenMatch : matches || childrenMatch) &&
    (depSelf || depChildrenMatch);

  // 搜索 / 协议 / Method 过滤 / 废弃筛选命中时自动展开包含命中项的文件夹，保证结果可见
  useEffect(() => {
    if (
      isFolder &&
      (((filter || protocolFilters.length > 0 || methodFilters.length > 0) &&
        childrenMatch) ||
        (depFilter !== "all" && depChildrenMatch))
    ) {
      onToggleOpen(node.path, true);
    }
  }, [filter, protocolFilters, methodFilters, isFolder, childrenMatch, depFilter, depChildrenMatch, node.path, onToggleOpen]);

  if (!visible) return null;

  const selected = selectedPath === node.path;
  const indent = depth * 14 + 6;
  // 文件夹行与接口行都是拖拽落点：落目录 = 移入；落同级 = 排序（插入其前/后）
  const dropTarget = node.path;
  const canDrop = !!dragSrc && validDrop(dragSrc, dropTarget, isFolder);
  // 同级排序（分组与接口均可）：显示插入位置指示线，而非整行高亮
  const sortDrop = canDrop && !!dragSrc && parentDir(dragSrc) === parentDir(dropTarget);
  const showDragOver = canDrop && dragOver === dropTarget && !sortDrop;

  return (
    <div>
      <div
        className={`node ${selected ? "selected" : ""} ${showDragOver ? "drag-over" : ""} ${dragSrc === node.path ? "dragging" : ""} ${isFolder ? "folder-node" : ""} ${deprecated ? "deprecated" : ""}`}
        style={{ paddingLeft: indent }}
        draggable={true}
        onDragStart={(e) => {
          e.stopPropagation();
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData("text/plain", node.path);
          onDragStart(node.path);
        }}
        onDragEnd={(e) => {
          e.stopPropagation();
          onDragEnd();
          setDropPos(null);
        }}
        onDragOver={(e) => {
          // 始终允许放置，避免出现禁止图标；有效性在 drop 时校验
          e.preventDefault();
          e.stopPropagation();
          e.dataTransfer.dropEffect = "move";
          if (canDrop) {
            onDragOverTarget(dropTarget);
            if (sortDrop) {
              // 落点在元素上半部 = 放前面，下半部 = 放后面
              const r = e.currentTarget.getBoundingClientRect();
              setDropPos(e.clientY < r.top + r.height / 2 ? "top" : "bottom");
            }
          }
        }}
        onDragLeave={(e) => {
          e.stopPropagation();
          // 仅当真正离开本行时清除高亮（移动到行内子元素不算离开）
          const rt = e.relatedTarget as Node | null;
          if (rt && e.currentTarget.contains(rt)) return;
          onDragLeaveTarget(dropTarget);
          setDropPos(null);
        }}
        onDrop={(e) => {
          e.preventDefault();
          e.stopPropagation();
          const src = e.dataTransfer.getData("text/plain") || dragSrc;
          if (src) onDropTarget(src, dropTarget, isFolder, dropPos === "bottom", node.order);
          setDropPos(null);
        }}
        onClick={() => {
          if (isFolder) onToggleOpen(node.path, !open);
          else onSelect(node);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onContextMenu(e, node);
        }}
        title={
          deprecated
            ? `${t("sidebar.deprecated")} · ${canDrop ? t("sidebar.dropHere") : isFolder ? node.description || node.name : `${isWs ? "WebSocket" : node.method} ${node.endpoint}`}`
            : canDrop
              ? t("sidebar.dropHere")
              : isFolder
                ? node.description || node.name
                : `${isWs ? "WebSocket" : node.method} ${node.endpoint}`
        }
      >
        {sortDrop && dragOver === dropTarget && dropPos && (
          <div className={`drop-indicator ${dropPos}`} />
        )}
        {isFolder ? (
          <span className={`caret ${open ? "open" : ""}`}>▶</span>
        ) : (
          <span className="caret"></span>
        )}
        <span className="node-icon">
          {isFolder ? (
            "📁"
          ) : node.protocol === "websocket" ? (
            <img className="node-type-icon" src={iconWs} alt="WS" />
          ) : node.protocol === "socketio" ? (
            <img className="node-type-icon" src={iconSocketIo} alt="Socket.IO" />
          ) : node.protocol === "graphql" ? (
            <img className="node-type-icon" src={iconGql} alt="GraphQL" />
          ) : (
            <img className="node-type-icon" src={iconHttp} alt="HTTP" />
          )}
        </span>
        {deprecated && (
          <span className="node-dep-badge" title={t("sidebar.deprecatedBadge")}>
            {t("sidebar.deprecated")}
          </span>
        )}
        <span className="node-name">{node.name}</span>
        {isFolder && !!node.apiCount && (
          <span className="node-count" title={t("sidebar.apiCount", { count: node.apiCount })}>
            {node.apiCount}
          </span>
        )}
        {!isFolder && node.endpoint && (
          <span className="node-endpoint" title={node.endpoint}>
            {node.endpoint}
          </span>
        )}
        {!isFolder && node.method && !isWs && (
          <span className={`node-method ${methodClass(node.method)}`}>{node.method}</span>
        )}
        {!isFolder && node.mockEnabled && <span className="mock-dot" title={t("sidebar.mockEnabled")} />}
        <span className="node-actions">
          {isFolder && (
            <button
              className="node-action"
              title={t("sidebar.newApiIn")}
              onClick={(e) => {
                e.stopPropagation();
                onNewApi(node.path);
              }}
            >
              +
            </button>
          )}
          <button
            className="node-action"
            title={isFolder ? t("sidebar.editInfo") : t("sidebar.rename")}
            onClick={(e) => {
              e.stopPropagation();
              if (isFolder) onEditInfo(node);
              else onRename(node);
            }}
          >
            ✎
          </button>
          <button
            className="node-action del"
            title={t("sidebar.delete")}
            onClick={(e) => {
              e.stopPropagation();
              onDelete(node);
            }}
          >
            🗑
          </button>
        </span>
      </div>
      {isFolder && open && node.children && (
        <div>
          {node.children.map((child) => (
            <NodeRow
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              openMap={openMap}
              onToggleOpen={onToggleOpen}
              onSelect={onSelect}
              onNewApi={onNewApi}
              onNewFolder={onNewFolder}
              onRename={onRename}
              onCopy={onCopy}
              onDelete={onDelete}
              onToggleDeprecated={onToggleDeprecated}
              onEditInfo={onEditInfo}
              onVersions={onVersions}
              onStats={onStats}
              onViewMarkdown={onViewMarkdown}
              onViewApiDoc={onViewApiDoc}
              enableVersion={enableVersion}
              onContextMenu={onContextMenu}
              filter={filter}
              protocolFilters={protocolFilters}
              methodFilters={methodFilters}
              depFilter={depFilter}
              depInherited={deprecated}
              tree={null}
              dragSrc={dragSrc}
              dragOver={dragOver}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
              onDragOverTarget={onDragOverTarget}
              onDragLeaveTarget={onDragLeaveTarget}
              onDropTarget={onDropTarget}
            />
          ))}
          {!filter && depFilter === "all" && (
            <div
              className="node"
              style={{ paddingLeft: indent + depth * 14 + 10, color: "var(--text-faint)", fontSize: 12 }}
              onClick={() => onNewFolder(node.path)}
            >
              ＋ {t("sidebar.newFolder")}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function Sidebar(props: Props) {
  const t = useT();
  const { tree, loading, genLogsRecords, genLogsLoading, genLogsSelected, onGenLogsSelect, onGenLogsReload, onNewApi, onNewFolder, onRename, onCopy, onDelete, onToggleDeprecated, onEditInfo, onVersions, onStats, onViewMarkdown, onOpenSettings, onOpenGenLogs, view, onSwitchView, onImportPostman, onImportCurl, onImportOpenApi, onImportMarkdown, onImportApifox, onImportApipost, onImportRaml, onImportWadl, onImportHar, onImportYapi, onImportEolink, onImportInsomnia, onImportJmeter, onImportApiDoc, onImportExtra, onExport, onExportNode, onViewApiDoc, vcs, onVcsSync, onVcsCommitPush, enableVersion, settings } = props;
  const [importMenu, setImportMenu] = useState(false);
  /** 对象管理：底部导入（建表语句 / 建表文件） */
  const [objImportMenu, setObjImportMenu] = useState(false);
  const [objDdlOpen, setObjDdlOpen] = useState(false);
  const [objDdlText, setObjDdlText] = useState("");
  const objFileRef = useRef<HTMLInputElement | null>(null);

  /** 建表语句导入：每个 CREATE TABLE 生成一个对象（放未分组） */
  const doObjImportDdl = async (ddl: string) => {
    const text = ddl.trim();
    if (!text) {
      props.onObjectsToast(t("objects.importDdlEmpty"));
      return;
    }
    try {
      const res = await props.onObjectsImportDdl("", text);
      if (res.created.length) props.onObjectsToast(t("objects.importCreated", { n: res.created.length }));
      if (res.reused.length) props.onObjectsToast(t("objects.importReused", { n: res.reused.length }));
    } catch (e) {
      props.onObjectsToast(String(e));
    }
  };

  /** 建表文件导入：按文件名（去扩展名）创建分组，对象导入到该分组下 */
  const doObjImportFile = async (f: File) => {
    const text = (await f.text()).trim();
    if (!text) {
      props.onObjectsToast(t("objects.importDdlEmpty"));
      return;
    }
    const groupName = (f.name || "").replace(/\.[^.\\/]*$/, "").trim();
    let groupId = "";
    if (groupName) {
      const existing = props.objectsStore.groups.find((g) => g.id === groupName || g.name === groupName);
      if (existing) {
        groupId = existing.id;
      } else {
        await props.onObjectsSave({
          groups: [...props.objectsStore.groups, { id: groupName, name: groupName, deprecated: false }],
          objects: props.objectsStore.objects,
        });
        groupId = groupName;
      }
    }
    try {
      const res = await props.onObjectsImportDdl(groupId, text);
      if (res.created.length) props.onObjectsToast(t("objects.importCreated", { n: res.created.length }));
      if (res.reused.length) props.onObjectsToast(t("objects.importReused", { n: res.reused.length }));
    } catch (e) {
      props.onObjectsToast(String(e));
    }
  };

  // 导入菜单打开时按 ESC 关闭
  useEffect(() => {
    if (!importMenu) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setImportMenu(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [importMenu]);
  /** 文件夹展开状态（path → open），跨 loadAll/导入刷新保留；并持久化到目录 __info.json（collapsed） */
  const [openMap, setOpenMap] = useState<Record<string, boolean>>({});
  const toggleOpen = useCallback((path: string, open: boolean) => {
    setOpenMap((m) => ({ ...m, [path]: open }));
    // 记录开闭状态，重开应用时按此状态显示
    void setFolderCollapsed(path, !open).catch(() => {});
  }, []);
  const [filter, setFilter] = useState("");
  /** 高级搜索：是否展开过滤面板 */
  const [advOpen, setAdvOpen] = useState(false);
  /** 高级搜索：按接口协议类型多选过滤（空数组 = 不过滤） */
  const [protocolFilters, setProtocolFilters] = useState<string[]>([]);
  /** 高级搜索：按接口 Method 多选过滤（空数组 = 不过滤） */
  const [methodFilters, setMethodFilters] = useState<string[]>([]);
  const [depFilter, setDepFilter] = useState<DepFilter>("all");
  const [menu, setMenu] = useState<CtxMenu | null>(null);
  const [bgMenu, setBgMenu] = useState<{ x: number; y: number } | null>(null);
  const [dragSrc, setDragSrc] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState<string | null>(null);

  const handleDragStart = (path: string) => setDragSrc(path);
  const handleDragEnd = () => {
    setDragSrc(null);
    setDragOver(null);
  };
  const handleDragOverTarget = (dst: string) => setDragOver(dst);
  const handleDragLeaveTarget = (dst: string) =>
    setDragOver((prev) => (prev === dst ? null : prev));
  const handleDropTarget = async (
    src: string,
    dst: string,
    dstIsFolder: boolean,
    after: boolean,
    dstOrder?: number,
  ) => {
    setDragSrc(null);
    setDragOver(null);
    if (!validDrop(src, dst, dstIsFolder)) return; // 无效落点（自身/子目录/原地）：忽略
    if (parentDir(src) === parentDir(dst)) {
      // 同级拖动排序：放前面 = 目标 order -1，放后面 = 目标 order +1
      const newOrder = (dstOrder ?? 0) + (after ? 1 : -1);
      if (props.onReorderOne) await props.onReorderOne(src, newOrder);
      return;
    }
    // 移动：落到目录 → 该目录；落到接口 → 该接口所在目录
    await props.onMove(src, dstIsFolder ? dst : parentDir(dst));
  };

  // 右键菜单：点击任意处 / Esc / 滚动时关闭
  useEffect(() => {
    if (!menu && !bgMenu) return;
    const close = () => {
      setMenu(null);
      setBgMenu(null);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu, bgMenu]);

  const openMenu = (e: React.MouseEvent, node: TreeNode) => {
    setMenu({
      x: Math.min(e.clientX, window.innerWidth - 190),
      y: Math.min(e.clientY, window.innerHeight - 160),
      node,
    });
  };

  return (
    <div className="sidebar" style={{ width: props.width ?? 310 }}>
      <div className="sidebar-header">
        {view === "api" ? (
          <>
            <div className="sidebar-search-row">
            <div className="search-box">
              <span className="icon">🔍</span>
              <input
                placeholder={t("sidebar.search")}
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                spellCheck={false}
              />
              {filter && (
                <button
                  className="search-clear"
                  onClick={() => setFilter("")}
                  title={t("common.clear")}
                  aria-label={t("common.clear")}
                >
                  <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
                    <path d="M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
                  </svg>
                </button>
              )}
              <button
                className={`search-adv-toggle${advOpen ? " on" : ""}`}
                onClick={() => setAdvOpen((s) => !s)}
                title={t("sidebar.advSearch")}
                aria-label={t("sidebar.advSearch")}
              >
                <svg viewBox="0 0 24 24" width="13" height="13" fill="currentColor" aria-hidden="true">
                  <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
                </svg>
              </button>
            </div>
            <select
              className="dep-filter"
              value={depFilter}
              onChange={(e) => setDepFilter(e.target.value as DepFilter)}
              title={t("sidebar.depFilterTip")}
            >
              <option value="all">{t("sidebar.depFilterAll")}</option>
              <option value="active">{t("sidebar.depFilterActive")}</option>
              <option value="deprecated">{t("sidebar.depFilterDeprecated")}</option>
            </select>
          </div>
          {advOpen && (
            <div className="adv-search">
              <div className="adv-search-title">{t("sidebar.advProtocolType")}</div>
              <div className="adv-methods">
                {PROTOCOL_OPTIONS.map((p) => {
                  const on = protocolFilters.includes(p.id);
                  return (
                    <label key={p.id} className={`adv-method${on ? " on" : ""}`}>
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() =>
                          setProtocolFilters((prev) =>
                            on ? prev.filter((x) => x !== p.id) : [...prev, p.id]
                          )
                        }
                      />
                      {p.label}
                    </label>
                  );
                })}
              </div>
              <div className="adv-search-title">{t("sidebar.advMethodType")}</div>
              <div className="adv-methods">
                {METHOD_OPTIONS.map((m) => {
                  const on = methodFilters.includes(m);
                  return (
                    <label key={m} className={`adv-method${on ? " on" : ""}`}>
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() =>
                          setMethodFilters((prev) =>
                            on ? prev.filter((x) => x !== m) : [...prev, m]
                          )
                        }
                      />
                      {m}
                    </label>
                  );
                })}
              </div>
              <div className="adv-search-actions">
                <button
                  className="btn-link"
                  onClick={() => {
                    setProtocolFilters(PROTOCOL_OPTIONS.map((p) => p.id));
                    setMethodFilters([...METHOD_OPTIONS]);
                  }}
                >
                  {t("common.selectAll")}
                </button>
                <button
                  className="btn-link"
                  onClick={() => {
                    setProtocolFilters([]);
                    setMethodFilters([]);
                  }}
                >
                  {t("common.clear")}
                </button>
              </div>
            </div>
          )}
          </>
        ) : view === "genlogs" ? (
          <div className="history-side-header">
            <span className="history-side-title">📄 {t("sidebar.genLogs")}</span>
            <button
              className="icon-btn"
              onClick={() => onSwitchView("api")}
              title={t("history.back")}
              aria-label={t("history.back")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M9.4 16.6 4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4zm5.2 0 4.6-4.6-4.6-4.6L16 6l6 6-6 6-1.4-1.4z" />
              </svg>
            </button>
          </div>
        ) : view === "objects" ? (
          <div className="history-side-header">
            <span className="history-side-title">🗂️ {t("objects.title")}</span>
            <button
              className="icon-btn"
              onClick={() => onSwitchView("api")}
              title={t("history.back")}
              aria-label={t("history.back")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M9.4 16.6 4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4zm5.2 0 4.6-4.6-4.6-4.6L16 6l6 6-6 6-1.4-1.4z" />
              </svg>
            </button>
          </div>
        ) : (
          <div className="history-side-header">
            <span className="history-side-title">{t("history.title")}</span>
            <button
              className="icon-btn"
              onClick={() => onSwitchView("api")}
              title={t("history.back")}
              aria-label={t("history.back")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M9.4 16.6 4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4zm5.2 0 4.6-4.6-4.6-4.6L16 6l6 6-6 6-1.4-1.4z" />
              </svg>
            </button>
          </div>
        )}
      </div>
      <div
        className={`tree ${dragOver === "__root__" ? "drag-over-root" : ""}`}
        style={{ display: view === "api" ? undefined : "none" }}
        onContextMenu={(e) => {
          // 空白处右键：新建接口 / 新建分组（节点行上的右键会 stopPropagation）
          e.preventDefault();
          setBgMenu({
            x: Math.min(e.clientX, window.innerWidth - 190),
            y: Math.min(e.clientY, window.innerHeight - 160),
          });
        }}
        onDragOver={(e) => {
          // 始终允许放置到根目录，避免禁止图标
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
          if (dragSrc && parentDir(dragSrc) !== "") setDragOver("__root__");
        }}
        onDragLeave={() => setDragOver((prev) => (prev === "__root__" ? null : prev))}
        onDrop={(e) => {
          e.preventDefault();
          const src = e.dataTransfer.getData("text/plain") || dragSrc;
          if (src) void props.onMove(src, "");
        }}
      >
        {tree && tree.children && (
          <div>
            {tree.children.map((child) => (
              <NodeRow
                key={child.path}
                node={child}
                depth={0}
                selectedPath={props.selectedPath}
                openMap={openMap}
                onToggleOpen={toggleOpen}
                onSelect={props.onSelect}
                onNewApi={onNewApi}
                onNewFolder={onNewFolder}
                onRename={onRename}
                onCopy={onCopy}
                onDelete={onDelete}
                onToggleDeprecated={onToggleDeprecated}
                onEditInfo={onEditInfo}
                onVersions={onVersions}
                onStats={props.onStats}
                enableVersion={enableVersion}
                onContextMenu={openMenu}
                filter={filter.trim().toLowerCase()}
                protocolFilters={protocolFilters}
                methodFilters={methodFilters}
                depFilter={depFilter}
                depInherited={false}
                tree={null}
                dragSrc={dragSrc}
                dragOver={dragOver}
                onDragStart={handleDragStart}
                onDragEnd={handleDragEnd}
                onDragOverTarget={handleDragOverTarget}
                onDragLeaveTarget={handleDragLeaveTarget}
                onDropTarget={handleDropTarget}
              />
            ))}
            {!filter.trim() && depFilter === "all" && (
              <div
                className="node"
                style={{ paddingLeft: 10, color: "var(--text-faint)", fontSize: 12 }}
                onClick={() => onNewFolder("")}
              >
                ＋ {t("sidebar.newFolder")}
              </div>
            )}
          </div>
        )}
        {loading && tree && (
          <div className="tree-loading-inline">
            <span className="spinner" />
          </div>
        )}
        {loading && !tree && (
          <div className="tree-loading">
            <span className="spinner" />
            <span>{t("sidebar.loading")}</span>
          </div>
        )}
        {!loading && !tree && <div className="tree-root">{t("sidebar.emptyTree")}</div>}
      </div>
      {view === "objects" && (
        <ObjectsTree
          store={props.objectsStore}
          usage={props.objectsUsage}
          onSave={props.onObjectsSave}
          onImport={props.onObjectsImport}
          onImportDdl={props.onObjectsImportDdl}
          onToast={props.onObjectsToast}
          selectedUuid={props.objectsSelectedUuid}
          onSelectObject={props.onObjectsSelect}
          newReq={props.objectsNewReq}
          importReq={props.objectsImportReq}
          defaultFolderState={settings?.defaultFolderState ?? "expanded"}
        />
      )}
      {view === "genlogs" && (
        <GenLogsList
          records={genLogsRecords}
          loading={genLogsLoading}
          selectedId={genLogsSelected}
          onSelect={onGenLogsSelect}
          onReload={onGenLogsReload}
        />
      )}
      {view === "history" && (
        <HistoryList
          records={props.historyRecords}
          days={props.historyDays}
          loading={props.historyLoading}
          hasMore={props.historyHasMore}
          selectedId={props.historySelected}
          totalCount={props.historyTotal}
          onSelect={props.onHistorySelect}
          onLoadMore={props.onHistoryLoadMore}
          onReload={props.onHistoryReload}
          onClear={props.onHistoryClear}
          diffMode={props.historyDiffMode}
          diffIds={props.historyDiffIds}
          diffError={props.historyDiffError}
          onToggleDiffMode={props.onHistoryToggleDiffMode}
          onToggleDiffSelect={props.onHistoryToggleDiffSelect}
          onStartDiff={props.onHistoryStartDiff}
        />
      )}
      <div className="sidebar-footer" onContextMenu={(e) => e.preventDefault()}>
        <button
          className={`icon-btn ${view === "api" ? "active" : ""}`}
          onClick={() => onSwitchView("api")}
          title={t("sidebar.api")}
          aria-label={t("sidebar.api")}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M14 12l-2 2-2-2 2-2 2 2zm-2-6 2.12 2.12 2.5-2.5L12 1 7.38 5.62l2.5 2.5L12 6zm-6 6 2.12-2.12-2.5-2.5L1 12l4.62 4.62 2.5-2.5L6 12zm12 0-2.12 2.12 2.5 2.5L23 12l-4.62-4.62-2.5 2.5L18 12zm-6 6-2.12-2.12-2.5 2.5L12 23l4.62-4.62-2.5-2.5L12 18z" />
          </svg>
        </button>
        <button
          className={`icon-btn ${view === "objects" ? "active" : ""}`}
          onClick={() => onSwitchView(view === "objects" ? "api" : "objects")}
          title={t("objects.title")}
          aria-label={t("objects.title")}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M21 7.6 12 3 3 7.6v8.8L12 21l9-4.6V7.6zM12 4.7l6.8 3.5-6.8 3.5-6.8-3.5L12 4.7zm-6.5 9.2v-4.7l5.7 2.9v4.7l-5.7-2.9zm7.3 2.9v-4.7l5.7-2.9v4.7l-5.7 2.9z" />
          </svg>
        </button>
        <button
          className={`icon-btn ${view === "history" ? "active" : ""}`}
          onClick={() => onSwitchView(view === "history" ? "api" : "history")}
          title={view === "history" ? t("history.back") : t("history.title")}
          aria-label={t("history.title")}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z" />
          </svg>
        </button>
        {vcs && (
          <>
            <button
              className="icon-btn"
              onClick={onVcsSync}
              title={t("vcs.syncTip", { vcs: vcs === "git" ? "git pull" : "svn update" })}
              aria-label={t("vcs.sync")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M17.65 6.35A7.95 7.95 0 0 0 12 4a8 8 0 1 0 7.73 10h-2.08A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" />
              </svg>
            </button>
            <button
              className="icon-btn"
              onClick={onVcsCommitPush}
              title={t("vcs.pushTip")}
              aria-label={t("vcs.push")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M5 20h14v-2H5v2zM12 4l-6 6h4v5h4v-5h4l-6-6z" />
              </svg>
            </button>
          </>
        )}
        <button
          className={`icon-btn ${view === "genlogs" ? "active" : ""}`}
          onClick={onOpenGenLogs}
          title={t("sidebar.genLogs")}
          aria-label={t("sidebar.genLogs")}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z" />
          </svg>
        </button>
        <button className="icon-btn" onClick={onOpenSettings} title={t("sidebar.settings")} aria-label={t("sidebar.settings")}>
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
            <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
          </svg>
        </button>
        {view === "objects" && (
          <button
            className="icon-btn objects-import-btn"
            style={{ marginLeft: "auto" }}
            onClick={() => setObjImportMenu(!objImportMenu)}
            title={t("objects.importFile")}
            aria-label={t("objects.importFile")}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
              <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z" />
            </svg>
          </button>
        )}
        {objImportMenu && (
          <>
            <div className="menu-mask" onClick={() => setObjImportMenu(false)} />
            <div className="import-menu objects-import-menu">
              <button
                onClick={() => {
                  setObjImportMenu(false);
                  setObjDdlText("");
                  setObjDdlOpen(true);
                }}
              >
                <span className="import-menu-icon">📝</span>
                {t("objects.importDdlText")}
              </button>
              <button
                onClick={() => {
                  setObjImportMenu(false);
                  objFileRef.current?.click();
                }}
              >
                <span className="import-menu-icon">📄</span>
                {t("objects.importDdlFile")}
              </button>
            </div>
          </>
        )}
        <input
          ref={objFileRef}
          type="file"
          accept=".sql,text/plain"
          style={{ display: "none" }}
          onChange={async (e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            if (!f) return;
            try {
              await doObjImportFile(f);
            } catch (err) {
              props.onObjectsToast(String(err));
            }
          }}
        />
        {view === "api" && onExport && settings?.exportEnabled !== false && (
          <button className="icon-btn" onClick={onExport} title={t("sidebar.export")} aria-label={t("sidebar.export")}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
              <path d="M4 12l1.41 1.41L11 7.83V20h2V7.83l5.58 5.59L20 12l-8-8-8 8z" />
            </svg>
          </button>
        )}
        {view === "api" && onImportPostman && settings?.importEnabled !== false && (
          <>
            <button
              className="icon-btn import-btn"
              onClick={() => setImportMenu(!importMenu)}
              title={t("sidebar.import")}
              aria-label={t("sidebar.import")}
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor" aria-hidden="true">
                <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z" />
              </svg>
            </button>
            {importMenu && (
              <>
                <div className="menu-mask" onClick={() => setImportMenu(false)} />
                <div className="import-menu">
                  {settings?.importTypes?.postman !== false && onImportPostman && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportPostman();
                      }}
                    >
                      <FormatIcon value="postman" className="import-menu-icon" />
                      Postman Collection
                    </button>
                  )}
                  {settings?.importTypes?.curl !== false && onImportCurl && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportCurl();
                      }}
                    >
                      <FormatIcon value="curl" className="import-menu-icon" />
                      {t("sidebar.importCurl")}
                    </button>
                  )}
                  {settings?.importTypes?.openapi !== false && onImportOpenApi && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportOpenApi();
                      }}
                    >
                      <FormatIcon value="openapi" className="import-menu-icon" />
                      OpenAPI / Swagger（JSON / YAML）
                    </button>
                  )}
                  {settings?.importTypes?.markdown !== false && onImportMarkdown && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportMarkdown();
                      }}
                    >
                      <FormatIcon value="markdown" className="import-menu-icon" />
                      Markdown {t("sidebar.markdown")}
                    </button>
                  )}
                  {settings?.importTypes?.apifox !== false && onImportApifox && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportApifox();
                      }}
                    >
                      <FormatIcon value="apifox" className="import-menu-icon" />
                      Apifox 项目（JSON）
                    </button>
                  )}
                  {settings?.importTypes?.apipost !== false && onImportApipost && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportApipost();
                      }}
                    >
                      <FormatIcon value="apipost" className="import-menu-icon" />
                      Apipost 项目（JSON）
                    </button>
                  )}
                  {settings?.importTypes?.raml !== false && onImportRaml && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportRaml();
                      }}
                    >
                      <FormatIcon value="raml" className="import-menu-icon" />
                      RAML 文档（YAML）
                    </button>
                  )}
                  {settings?.importTypes?.wadl !== false && onImportWadl && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportWadl();
                      }}
                    >
                      <FormatIcon value="wadl" className="import-menu-icon" />
                      WADL 文档（XML）
                    </button>
                  )}
                  {settings?.importTypes?.har !== false && onImportHar && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportHar();
                      }}
                    >
                      <FormatIcon value="har" className="import-menu-icon" />
                      HAR 抓包文件
                    </button>
                  )}
                  {settings?.importTypes?.yapi !== false && onImportYapi && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportYapi();
                      }}
                    >
                      <FormatIcon value="yapi" className="import-menu-icon" />
                      YApi 项目导出
                    </button>
                  )}
                  {settings?.importTypes?.eolink !== false && onImportEolink && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportEolink();
                      }}
                    >
                      <FormatIcon value="eolink" className="import-menu-icon" />
                      Eolink 项目
                    </button>
                  )}
                  {settings?.importTypes?.insomnia !== false && onImportInsomnia && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportInsomnia();
                      }}
                    >
                      <FormatIcon value="insomnia" className="import-menu-icon" />
                      Insomnia 集合
                    </button>
                  )}
                  {settings?.importTypes?.jmeter !== false && onImportJmeter && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportJmeter();
                      }}
                    >
                      <FormatIcon value="jmeter" className="import-menu-icon" />
                      JMeter 测试计划
                    </button>
                  )}
                  {settings?.importTypes?.apidoc !== false && onImportApiDoc && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportApiDoc();
                      }}
                    >
                      <FormatIcon value="apidoc" className="import-menu-icon" />
                      apiDoc 文档
                    </button>
                  )}
                  {settings?.importTypes?.apidog !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("apidog");
                      }}
                    >
                      <FormatIcon value="apidog" className="import-menu-icon" />
                      apiDog
                    </button>
                  )}
                  {settings?.importTypes?.bruno !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("bruno");
                      }}
                    >
                      <FormatIcon value="bruno" className="import-menu-icon" />
                      Bruno
                    </button>
                  )}
                  {settings?.importTypes?.apizza !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("apizza");
                      }}
                    >
                      <FormatIcon value="apizza" className="import-menu-icon" />
                      Apizza
                    </button>
                  )}
                  {settings?.importTypes?.nei !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("nei");
                      }}
                    >
                      <FormatIcon value="nei" className="import-menu-icon" />
                      NEI
                    </button>
                  )}
                  {settings?.importTypes?.doclever !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("doclever");
                      }}
                    >
                      <FormatIcon value="doclever" className="import-menu-icon" />
                      DOClever
                    </button>
                  )}
                  {settings?.importTypes?.["io-docs"] !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("io-docs");
                      }}
                    >
                      <FormatIcon value="io-docs" className="import-menu-icon" />
                      IO-Docs
                    </button>
                  )}
                  {settings?.importTypes?.easydoc !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("easydoc");
                      }}
                    >
                      <FormatIcon value="easydoc" className="import-menu-icon" />
                      EasyDoc
                    </button>
                  )}
                  {settings?.importTypes?.docway !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("docway");
                      }}
                    >
                      <FormatIcon value="docway" className="import-menu-icon" />
                      DocWay
                    </button>
                  )}
                  {settings?.importTypes?.hoppscotch !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("hoppscotch");
                      }}
                    >
                      <FormatIcon value="hoppscotch" className="import-menu-icon" />
                      Hoppscotch
                    </button>
                  )}
                  {settings?.importTypes?.metersphere !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("metersphere");
                      }}
                    >
                      <FormatIcon value="metersphere" className="import-menu-icon" />
                      MeterSphere
                    </button>
                  )}
                  {settings?.importTypes?.rap2 !== false && onImportExtra && (
                    <button
                      onClick={() => {
                        setImportMenu(false);
                        onImportExtra("rap2");
                      }}
                    >
                      <FormatIcon value="rap2" className="import-menu-icon" />
                      RAP2
                    </button>
                  )}
                </div>
              </>
            )}
          </>
        )}
      </div>

      {menu && (
        <div className="node-ctx-menu" style={{ left: menu.x, top: menu.y }}>
          {menu.node.kind === "folder" ? (
            <>
              <button
                onClick={() => {
                  onNewFolder(menu.node.path);
                  setMenu(null);
                }}
              >
                📁 {t("sidebar.newFolder")}
              </button>
              <button
                onClick={() => {
                  onNewApi(menu.node.path);
                  setMenu(null);
                }}
              >
                🌐 {t("sidebar.newApi")}
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onExportNode?.(menu.node);
                  setMenu(null);
                }}
              >
                📤 {t("sidebar.exportNode")}
              </button>
              {onViewMarkdown && (
                <button
                  onClick={() => {
                    onViewMarkdown(menu.node);
                    setMenu(null);
                  }}
                >
                  📝 {t("sidebar.viewMarkdown")}
                </button>
              )}
              <button
                onClick={() => {
                  onCopy(menu.node);
                  setMenu(null);
                }}
              >
                📋 {t("sidebar.copy")}
              </button>
              <button
                onClick={() => {
                  onStats?.(menu.node);
                  setMenu(null);
                }}
              >
                📊 {t("sidebar.stats")}
              </button>
              <button
                onClick={() => {
                  onEditInfo(menu.node);
                  setMenu(null);
                }}
              >
                ✎ {t("sidebar.editInfo")}
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onToggleDeprecated(menu.node);
                  setMenu(null);
                }}
              >
                {menu.node.deprecated ? "✅ " : "🚫 "}
                {menu.node.deprecated
                  ? t("sidebar.unmarkDeprecated")
                  : t("sidebar.markDeprecated")}
              </button>
              <button
                className="danger"
                onClick={() => {
                  onDelete(menu.node);
                  setMenu(null);
                }}
              >
                🗑 {t("sidebar.delete")}
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => {
                  onRename(menu.node);
                  setMenu(null);
                }}
              >
                ✎ {t("sidebar.rename")}
              </button>
              {enableVersion && (
                <button
                  onClick={() => {
                    onVersions(menu.node);
                    setMenu(null);
                  }}
                >
                  📑 {t("sidebar.versions")}
                </button>
              )}
              {onViewMarkdown && (
                <button
                  onClick={() => {
                    onViewMarkdown(menu.node);
                    setMenu(null);
                  }}
                >
                  📝 {t("sidebar.viewMarkdown")}
                </button>
              )}
              {onViewApiDoc && (
                <button
                  onClick={() => {
                    onViewApiDoc(menu.node);
                    setMenu(null);
                  }}
                >
                  📄 {t("sidebar.viewApiDoc")}
                </button>
              )}
              <button
                onClick={() => {
                  onExportNode?.(menu.node);
                  setMenu(null);
                }}
              >
                📤 {t("sidebar.exportNode")}
              </button>
              <button
                onClick={() => {
                  onCopy(menu.node);
                  setMenu(null);
                }}
              >
                📋 {t("sidebar.copy")}
              </button>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  onToggleDeprecated(menu.node);
                  setMenu(null);
                }}
              >
                {menu.node.deprecated ? "✅ " : "🚫 "}
                {menu.node.deprecated
                  ? t("sidebar.unmarkDeprecated")
                  : t("sidebar.markDeprecated")}
              </button>
              <button
                className="danger"
                onClick={() => {
                  onDelete(menu.node);
                  setMenu(null);
                }}
              >
                🗑 {t("sidebar.delete")}
              </button>
            </>
          )}
        </div>
      )}

      {bgMenu && (
        <div className="node-ctx-menu" style={{ left: bgMenu.x, top: bgMenu.y }}>
          <button
            onClick={() => {
              onNewApi("");
              setBgMenu(null);
            }}
          >
            🌐 {t("sidebar.newApi")}
          </button>
          <button
            onClick={() => {
              onNewFolder("");
              setBgMenu(null);
            }}
          >
            📁 {t("sidebar.newFolder")}
          </button>
          {tree && (
            <>
              <div className="node-ctx-sep" />
              <button
                onClick={() => {
                  // 空白处右键 → 统计整个工作区（根目录）
                  onStats?.(tree);
                  setBgMenu(null);
                }}
              >
                📊 {t("sidebar.stats")}
              </button>
            </>
          )}
        </div>
      )}

      {/* 对象管理：从建表语句导入弹窗 */}
      {objDdlOpen && (
        <div className="objects-import-mask" onClick={() => setObjDdlOpen(false)}>
          <div className="objects-import-modal" onClick={(e) => e.stopPropagation()}>
            <div className="objects-import-title">{t("objects.importDdlLabel")}</div>
            <div className="objects-import-body">
              <textarea
                className="ddl-input"
                rows={10}
                value={objDdlText}
                onChange={(e) => setObjDdlText(e.target.value)}
                placeholder="CREATE TABLE user (&#10;  id BIGINT PRIMARY KEY,&#10;  name VARCHAR(64) NOT NULL&#10;);"
                spellCheck={false}
                autoFocus
              />
              <div className="objects-import-tip">{t("objects.importDdlTip")}</div>
            </div>
            <div className="objects-import-actions">
              <button className="btn" onClick={() => setObjDdlOpen(false)}>
                {t("common.cancel")}
              </button>
              <button
                className="btn primary"
                onClick={() => {
                  const d = objDdlText;
                  setObjDdlOpen(false);
                  setObjDdlText("");
                  void doObjImportDdl(d);
                }}
              >
                {t("common.confirm")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
