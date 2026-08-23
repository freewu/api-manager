import { useEffect, useMemo, useRef, useState } from "react";
import { ObjectDef, ObjectImportResult, ObjectProp, ObjectStore, ObjectUsageItem } from "../types";
import { useT } from "../i18n";
import { ObjectVersionModal } from "./ObjectVersionModal";

interface Props {
  store: ObjectStore;
  usage: ObjectUsageItem[];
  onSave: (store: ObjectStore) => Promise<ObjectStore>;
  onImport: (name: string, group: string, json: string) => Promise<ObjectImportResult>;
  onImportDdl: (group: string, ddl: string) => Promise<ObjectImportResult>;
  onToast: (msg: string) => void;
  /** 当前选中对象 uuid（受控，右侧展开配置；uuid 为稳定唯一标识） */
  selectedUuid: string | null;
  onSelectObject: (uuid: string | null) => void;
}

/** 对象管理：左侧树形目录（分组 = 目录，多级；不显示根目录） */
export default function ObjectsTree({
  store,
  usage,
  onSave,
  onImport,
  onImportDdl,
  onToast,
  selectedUuid,
  onSelectObject,
}: Props) {
  const t = useT();
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  /** 删除确认弹窗（对象 / 分组） */
  const [confirmDel, setConfirmDel] = useState<{ title: string; message: string; onConfirm: () => void } | null>(null);
  /** 新增对象弹窗（名称 + JSON，JSON 可为空） */
  const [newOpen, setNewOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [newJson, setNewJson] = useState("");
  const [newGroup, setNewGroup] = useState("");
  /** 新建分组弹窗 */
  const [groupOpen, setGroupOpen] = useState(false);
  const [groupName, setGroupName] = useState("");
  /** 文件导入（.json / .sql） */
  const fileRef = useRef<HTMLInputElement | null>(null);
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null);
  /** 对象行右键菜单（uuid） */
  const [objMenu, setObjMenu] = useState<{ x: number; y: number; uuid: string } | null>(null);
  /** 版本查看弹窗（对象 uuid） */
  const [versionModal, setVersionModal] = useState<ObjectDef | null>(null);
  // 拖拽中的对象 uuid
  const [dragUuid, setDragUuid] = useState<string | null>(null);

  // 首次加载默认展开第一层分组（父级展开状态便于发现层级；store 异步就绪后生效）
  const inited = useRef(false);
  useEffect(() => {
    if (store.groups.length === 0 || inited.current) return;
    inited.current = true;
    setOpenGroups(new Set(groupTree.map((g) => g.id)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store.groups]);

  // 右键菜单：点击任意处 / Esc 时关闭
  useEffect(() => {
    if (!ctxMenu && !objMenu) return;
    const close = () => {
      setCtxMenu(null);
      setObjMenu(null);
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
  }, [ctxMenu, objMenu]);

  const usageOf = useMemo(() => {
    const m: Record<string, ObjectUsageItem> = {};
    for (const u of usage) m[u.hash] = u;
    return m;
  }, [usage]);

  const kw = search.trim().toLowerCase();
  const filterMatch = (o: ObjectDef) => {
    if (!kw) return true;
    if (o.name.toLowerCase().includes(kw)) return true;
    const grp = store.groups.find((g) => g.id === o.group);
    if (grp && grp.name.toLowerCase().includes(kw)) return true;
    return false;
  };

  // 分组树：id 即路径（"父/子"），不显示根目录
  interface GNode {
    id: string;
    name: string;
    children: GNode[];
  }
  const groupTree = useMemo(() => {
    const roots: GNode[] = [];
    const byId: Record<string, GNode> = {};
    const ensure = (id: string): GNode => {
      if (byId[id]) return byId[id];
      const node: GNode = {
        id,
        name: id.split("/").pop() || id,
        children: [],
      };
      byId[id] = node;
      const p = id.lastIndexOf("/");
      if (p > 0) {
        ensure(id.slice(0, p)).children.push(node);
      } else {
        roots.push(node);
      }
      return node;
    };
    for (const g of store.groups) ensure(g.id);
    return roots;
  }, [store.groups]);

  const objectsByGroup = useMemo(() => {
    const m: Record<string, ObjectDef[]> = { "": [] };
    for (const g of store.groups) m[g.id] = [];
    for (const o of store.objects) (m[o.group] ||= []).push(o);
    return m;
  }, [store]);

  const groupHasMatch = (g: GNode): boolean => {
    const items = objectsByGroup[g.id] || [];
    if (items.some(filterMatch)) return true;
    return g.children.some(groupHasMatch);
  };

  // ===== 保存：回读后端权威 store（hash 重算），按名称维持选中 =====
  const saveStore = async (next: ObjectStore) => {
    const fresh = await onSave(next);
    if (selectedUuid) {
      const prev = store.objects.find((o) => o.uuid === selectedUuid);
      if (prev) {
        const updated = fresh.objects.find((o) => o.uuid === prev.uuid);
        if (updated && updated.uuid !== prev.uuid) onSelectObject(updated.uuid);
      }
    }
    return fresh;
  };

  const renderGroup = (g: GNode, depth: number) => {
    const items = (objectsByGroup[g.id] || []).filter(filterMatch);
    const isOpen = openGroups.has(g.id) || !!kw;
    const nameMatch = !kw || g.name.toLowerCase().includes(kw);
    const childMatch = g.children.some(groupHasMatch);
    if (kw && !nameMatch && items.length === 0 && !childMatch) return null;
    return (
      <div key={g.id}>
        <div
          className="node objects-group-row"
          style={{ padding: "5px 6px 5px " + (4 + depth * 4) + "px" }}
          onClick={() =>
            setOpenGroups((prev) => {
              const next = new Set(prev);
              if (next.has(g.id)) next.delete(g.id);
              else next.add(g.id);
              return next;
            })
          }
          onDragOver={(e) => {
            if (dragUuid) e.preventDefault();
          }}
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (dragUuid) moveObject(dragUuid, g.id);
          }}
        >
          <span className={`caret${isOpen ? " open" : ""}`}>{isOpen ? "▾" : "▸"}</span>
          <span className="node-icon">{isOpen ? "📂" : "📁"}</span>
          <span className="node-name">{g.name}</span>
          <span
            className="objects-group-count"
            title={t("objects.groupCount", { count: items.length })}
          >
            {items.length}
          </span>
          <span className="objects-group-ops">
            <button
              className="icon-btn"
              title={t("objects.newObjectInGroup")}
              onClick={(e) => {
                e.stopPropagation();
                openNewObject(g.id);
              }}
            >
              ＋
            </button>
            <button
              className="icon-btn"
              title={t("objects.renameGroup")}
              onClick={(e) => {
                e.stopPropagation();
                renameGroup(g.id, g.name);
              }}
            >
              ✎
            </button>
            <button
              className="icon-btn"
              title={t("objects.deleteGroup")}
              onClick={(e) => {
                e.stopPropagation();
                deleteGroup(g.id);
              }}
            >
              🗑
            </button>
          </span>
        </div>
        {isOpen && (
          <>
            {items.map((o) => (
              <ObjectRow
                key={o.uuid}
                obj={o}
                depth={depth + 1}
                usageCount={usageOf[o.hash]?.apiCount ?? 0}
                selected={selectedUuid === o.uuid}
                onSelect={() => onSelectObject(o.uuid)}
                onRename={() => renameObject(o)}
                onDelete={() => deleteObject(o)}
                onContextMenu={(e, uuid) => {
                  e.preventDefault();
                  e.stopPropagation();
                  setObjMenu({ x: e.clientX, y: e.clientY, uuid });
                }}
                onDragStart={() => setDragUuid(o.uuid)}
                onDragEnd={() => setDragUuid(null)}
              />
            ))}
            {g.children.map((c) => renderGroup(c, depth + 1))}
          </>
        )}
      </div>
    );
  };

  // ===== 增删改（构造新 store 保存，目录结构即分组） =====
  const moveObject = (uuid: string, groupId: string) => {
    const o = store.objects.find((x) => x.uuid === uuid);
    if (!o || o.group === groupId) return;
    void saveStore({
      groups: store.groups,
      objects: store.objects.map((x) => (x.uuid === uuid ? { ...x, group: groupId } : x)),
    });
  };

  const openNewGroup = () => {
    setGroupName("");
    setGroupOpen(true);
  };

  const doNewGroup = async () => {
    const id = groupName.trim();
    if (!id) {
      onToast(t("objects.newGroupNameEmpty"));
      return;
    }
    if (store.groups.some((g) => g.id === id)) {
      onToast(t("objects.groupExists"));
      return;
    }
    setGroupOpen(false);
    await saveStore({
      groups: [...store.groups, { id, name: id.split("/").pop() || id }],
      objects: store.objects,
    });
    // 新建后自动展开该分组
    setOpenGroups((prev) => new Set(prev).add(id));
  };

  // 展开全部 / 收起全部
  const expandAll = () => {
    const all = new Set<string>();
    const walk = (nodes: GNode[]) => {
      for (const n of nodes) {
        all.add(n.id);
        walk(n.children);
      }
    };
    walk(groupTree);
    setOpenGroups(all);
  };
  const collapseAll = () => setOpenGroups(new Set());
  const allExpanded =
    groupTree.length > 0 && groupTree.every((g) => openGroups.has(g.id));

  /** 文件导入：.json → 按 JSON 导入（对象名=文件名）；.sql → 解析库名分组导入 */
  const onPickFile = (f: File) => {
    const reader = new FileReader();
    reader.onload = async () => {
      const text = String(reader.result || "");
      try {
        if (f.name.toLowerCase().endsWith(".sql")) {
          const m = text.match(/(?:use|create\s+database)\s+[`"]?([\w-]+)[`"]?\s*;?/i);
          const db = m ? m[1] : f.name.replace(/\.sql$/i, "");
          const res = await onImportDdl(db, text);
          finishImport(res);
        } else {
          JSON.parse(text);
          const name = f.name.replace(/\.json$/i, "") || "Object";
          const res = await onImport(name, "", text);
          finishImport(res);
        }
      } catch {
        onToast(t("objects.jsonInvalid"));
      }
    };
    reader.readAsText(f);
  };

  const renameGroup = (id: string, oldName: string) => {
    const name = window.prompt(t("objects.renameGroup"), oldName);
    if (!name || !name.trim() || name.trim() === oldName) return;
    const p = id.lastIndexOf("/");
    const newId = p > 0 ? `${id.slice(0, p)}/${name.trim()}` : name.trim();
    if (newId === id) return;
    // 更新自身与子分组 id（前缀替换），并同步对象 group
    const groups = store.groups.map((g) =>
      g.id === id || g.id.startsWith(id + "/")
        ? { ...g, id: newId + g.id.slice(id.length) }
        : g
    );
    const objects = store.objects.map((o) =>
      o.group === id || o.group.startsWith(id + "/")
        ? { ...o, group: newId + o.group.slice(id.length) }
        : o
    );
    void saveStore({ groups, objects });
  };

  const deleteGroup = (id: string) => {
    const name = id.split("/").pop() || id;
    setConfirmDel({
      title: t("objects.confirmTitle"),
      message: t("objects.confirmDeleteGroup", { name }),
      onConfirm: () => {
        const groups = store.groups.filter((g) => g.id !== id && !g.id.startsWith(id + "/"));
        const objects = store.objects.map((o) =>
          o.group === id || o.group.startsWith(id + "/") ? { ...o, group: "" } : o
        );
        void saveStore({ groups, objects });
      },
    });
  };

  /** 打开「新增对象」弹窗（groupId 为该对象所属分组，可为空串 = 未分组） */
  const openNewObject = (groupId: string) => {
    setNewGroup(groupId);
    setNewName("");
    setNewJson("");
    setNewOpen(true);
  };

  /** 弹窗确认：名称必填，JSON 可选（空 = 创建空对象） */
  const doNewObject = async () => {
    const name = newName.trim();
    if (!name) {
      onToast(t("objects.importName"));
      return;
    }
    let props: ObjectProp[] = [];
    if (newJson.trim()) {
      try {
        props = jsonToObjectProps(JSON.parse(newJson));
      } catch {
        onToast(t("objects.jsonInvalid"));
        return;
      }
    }
    setNewOpen(false);
    const now = Math.floor(Date.now() / 1000);
    const o: ObjectDef = {
      uuid: crypto.randomUUID(),
      hash: `tmp${Date.now().toString(36)}`,
      name,
      group: newGroup,
      description: "",
      properties: props,
      createdAt: now,
      updatedAt: now,
    };
    const fresh = await saveStore({ groups: store.groups, objects: [o, ...store.objects] });
    // 新建后直接在右侧展开配置
    const created = fresh.objects.find((x) => x.name === name);
    if (created) onSelectObject(created.uuid);
  };

  const renameObject = (o: ObjectDef) => {
    const name = window.prompt(t("objects.renameObject"), o.name);
    if (!name || !name.trim() || name.trim() === o.name) return;
    void saveStore({
      groups: store.groups,
      objects: store.objects.map((x) => (x.uuid === o.uuid ? { ...x, name: name.trim() } : x)),
    });
  };

  const deleteObject = (o: ObjectDef) => {
    setConfirmDel({
      title: t("objects.confirmTitle"),
      message: t("objects.confirmDelete", { name: o.name }),
      onConfirm: () => {
        // 按稳定 uuid 删除（hash 为结构签名可能重复，按 hash 删除会误伤同结构对象）
        void saveStore({
          groups: store.groups,
          objects: store.objects
            .filter((x) => x.uuid !== o.uuid)
            .map((x) => ({
              ...x,
              properties: x.properties.map((p) => (p.refHash === o.hash ? { ...p, refHash: "" } : p)),
            })),
        });
        if (selectedUuid === o.uuid) onSelectObject(null);
      },
    });
  };

  // ===== 文件导入结果处理（.json / .sql） =====
  const finishImport = (res: ObjectImportResult) => {
    const msgs: string[] = [];
    if (res.created.length) msgs.push(t("objects.importCreated", { n: res.created.length }));
    if (res.reused.length) msgs.push(t("objects.importReused", { n: res.reused.length }));
    onToast(msgs.join("，"));
    const top = res.topHash || (res.objects[0] && res.objects[0].hash) || null;
    if (top) {
      const found = store.objects.find((x) => x.hash === top);
      if (found) onSelectObject(found.uuid);
    }
  };

  return (
    <div className="history-list-side">
      <div className="history-side-toolbar">
        <div className="search-box objects-search-box">
          <span className="objects-search-icon">🔍</span>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t("objects.searchPlaceholder")}
            spellCheck={false}
          />
        </div>
      </div>
      <div
        className="tree objects-list"
        onContextMenu={(e) => {
          e.preventDefault();
          setCtxMenu({ x: Math.min(e.clientX, window.innerWidth - 190), y: Math.min(e.clientY, window.innerHeight - 120) });
        }}
      >
        {store.objects.length === 0 && <div className="objects-empty">{t("objects.empty")}</div>}
        {groupTree.map((g) => renderGroup(g, 0))}
        {/* 未分组对象：直接顶层叶子，不显示根目录 */}
        {!kw &&
          (objectsByGroup[""] || []).map((o) => (
            <ObjectRow
              key={o.uuid}
              obj={o}
              depth={0}
              usageCount={usageOf[o.hash]?.apiCount ?? 0}
              selected={selectedUuid === o.uuid}
              onSelect={() => onSelectObject(o.uuid)}
              onRename={() => renameObject(o)}
              onDelete={() => deleteObject(o)}
              onContextMenu={(e, uuid) => {
                e.preventDefault();
                e.stopPropagation();
                setObjMenu({ x: e.clientX, y: e.clientY, uuid });
              }}
              onDragStart={() => setDragUuid(o.uuid)}
              onDragEnd={() => setDragUuid(null)}
            />
          ))}
        {kw &&
          (objectsByGroup[""] || []).filter(filterMatch).map((o) => (
            <ObjectRow
              key={o.uuid}
              obj={o}
              depth={0}
              usageCount={usageOf[o.hash]?.apiCount ?? 0}
              selected={selectedUuid === o.uuid}
              onSelect={() => onSelectObject(o.uuid)}
              onRename={() => renameObject(o)}
              onDelete={() => deleteObject(o)}
              onContextMenu={(e, uuid) => {
                e.preventDefault();
                e.stopPropagation();
                setObjMenu({ x: e.clientX, y: e.clientY, uuid });
              }}
              onDragStart={() => setDragUuid(o.uuid)}
              onDragEnd={() => setDragUuid(null)}
            />
          ))}
        {!kw && (
          <>
            <div
              className="node"
              style={{ padding: "5px 6px 5px 6px", color: "var(--text-faint)", fontSize: 12 }}
              onClick={() => openNewObject("")}
            >
              ＋ {t("objects.newObject")}
            </div>
            <div
              className="node"
              style={{ padding: "5px 6px 5px 6px", color: "var(--text-faint)", fontSize: 12 }}
              onClick={openNewGroup}
            >
              ＋ {t("objects.newGroup")}
            </div>
          </>
        )}
      </div>

      {/* 底部工具栏：左 = 展开/收起，右 = 文件导入（.json / .sql） */}
      <div className="objects-side-footer">
        <button
          className="icon-btn objects-expand-btn"
          onClick={allExpanded ? collapseAll : expandAll}
          title={allExpanded ? t("objects.collapseAll") : t("objects.expandAll")}
        >
          {allExpanded ? "⤴" : "⤵"}
        </button>
        <input
          ref={fileRef}
          type="file"
          accept=".json,.sql"
          style={{ display: "none" }}
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) onPickFile(f);
            e.target.value = "";
          }}
        />
        <button className="objects-import-file-btn" onClick={() => fileRef.current?.click()}>
          <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor" aria-hidden="true">
            <path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z" />
          </svg>
          <span>{t("objects.importFile")}</span>
        </button>
      </div>

      {/* 对象行右键菜单：版本查看 / 重命名 / 删除 */}
      {objMenu && (
        <div className="node-ctx-menu" style={{ left: objMenu.x, top: objMenu.y }}>
          <button
            onClick={() => {
              const o = store.objects.find((x) => x.uuid === objMenu.uuid);
              setObjMenu(null);
              if (o) setVersionModal(o);
            }}
          >
            📑 {t("version.title")}
          </button>
          <button
            onClick={() => {
              const o = store.objects.find((x) => x.uuid === objMenu.uuid);
              setObjMenu(null);
              if (o) renameObject(o);
            }}
          >
            ✎ {t("objects.renameObject")}
          </button>
          <button
            className="danger"
            onClick={() => {
              const o = store.objects.find((x) => x.uuid === objMenu.uuid);
              setObjMenu(null);
              if (o) deleteObject(o);
            }}
          >
            🗑 {t("objects.deleteObject")}
          </button>
        </div>
      )}

      {/* 空白处右键菜单：新增分组 / 新增对象 */}
      {ctxMenu && (
        <div className="node-ctx-menu" style={{ left: ctxMenu.x, top: ctxMenu.y }}>
          <button
            onClick={() => {
              setCtxMenu(null);
              openNewGroup();
            }}
          >
            📁 {t("objects.newGroup")}
          </button>
          <button
            onClick={() => {
              setCtxMenu(null);
              openNewObject("");
            }}
          >
            ▦ {t("objects.newObject")}
          </button>
        </div>
      )}

      {/* 删除确认弹窗（对象 / 分组） */}
      {confirmDel && (
        <div className="objects-import-mask" onClick={() => setConfirmDel(null)}>
          <div className="objects-import-modal objects-confirm-modal" onClick={(e) => e.stopPropagation()}>
            <div className="objects-import-title">{confirmDel.title}</div>
            <div className="objects-confirm-body">{confirmDel.message}</div>
            <div className="objects-import-actions">
              <button className="btn" onClick={() => setConfirmDel(null)}>
                {t("common.cancel")}
              </button>
              <button
                className="btn danger"
                onClick={() => {
                  confirmDel.onConfirm();
                  setConfirmDel(null);
                }}
              >
                {t("common.delete")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 新建分组弹窗 */}
      {groupOpen && (
        <div className="objects-import-mask" onClick={() => setGroupOpen(false)}>
          <div className="objects-import-modal" onClick={(e) => e.stopPropagation()}>
            <div className="objects-import-title">{t("objects.newGroupTitle")}</div>
            <div className="objects-import-body">
              <label>
                <span>{t("objects.newGroup")}</span>
                <input
                  value={groupName}
                  onChange={(e) => setGroupName(e.target.value)}
                  placeholder="用户管理"
                  autoFocus
                  spellCheck={false}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void doNewGroup();
                  }}
                />
              </label>
              <div className="objects-import-tip">{t("objects.newGroupTip")}</div>
            </div>
            <div className="objects-import-actions">
              <button className="btn" onClick={() => setGroupOpen(false)}>
                {t("common.cancel")}
              </button>
              <button className="btn primary" onClick={() => void doNewGroup()}>
                {t("common.confirm")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 新增对象弹窗：名称 + JSON（可为空） */}
      {newOpen && (
        <div className="objects-import-mask" onClick={() => setNewOpen(false)}>
          <div className="objects-import-modal" onClick={(e) => e.stopPropagation()}>
            <div className="objects-import-title">{t("objects.newObjectTitle")}</div>
            <div className="objects-import-body">
              <label>
                <span>{t("objects.newObjectName")}</span>
                <input
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="User"
                  autoFocus
                  spellCheck={false}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void doNewObject();
                  }}
                />
              </label>
              <label>
                <span>{t("objects.importGroup")}</span>
                <select value={newGroup} onChange={(e) => setNewGroup(e.target.value)}>
                  <option value="">{t("objects.ungrouped")}</option>
                  {store.groups.map((g) => (
                    <option key={g.id} value={g.id}>
                      {g.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>{t("objects.newObjectJson")}</span>
                <textarea
                  value={newJson}
                  onChange={(e) => setNewJson(e.target.value)}
                  rows={7}
                  spellCheck={false}
                  placeholder={'{\n  "id": 1,\n  "name": "alice"\n}'}
                />
              </label>
              <div className="objects-import-tip">{t("objects.newObjectJsonTip")}</div>
            </div>
            <div className="objects-import-actions">
              <button className="btn" onClick={() => setNewOpen(false)}>
                {t("common.cancel")}
              </button>
              <button className="btn primary" onClick={() => void doNewObject()}>
                {t("common.confirm")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 对象版本查看弹窗 */}
      {versionModal && (
        <ObjectVersionModal current={versionModal} onClose={() => setVersionModal(null)} />
      )}
    </div>
  );
}

function ObjectRow({
  obj,
  depth,
  usageCount,
  selected,
  onSelect,
  onRename,
  onDelete,
  onContextMenu,
  onDragStart,
  onDragEnd,
}: {
  obj: ObjectDef;
  depth: number;
  usageCount: number;
  selected: boolean;
  onSelect: () => void;
  onRename: () => void;
  onDelete: () => void;
  onContextMenu: (e: React.MouseEvent, uuid: string) => void;
  onDragStart: () => void;
  onDragEnd: () => void;
}) {
  const t = useT();
  return (
    <div
      className={`node objects-object-row${selected ? " selected" : ""}`}
      style={{ padding: "5px 6px 5px " + (4 + depth * 4) + "px" }}
      onClick={onSelect}
      onContextMenu={(e) => onContextMenu(e, obj.uuid)}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("text/plain", obj.uuid);
        e.dataTransfer.effectAllowed = "move";
        onDragStart();
      }}
      onDragEnd={onDragEnd}
    >
      <span className="node-icon objects-object-icon">▦</span>
      <span className="node-name objects-object-name">{obj.name}</span>
      {usageCount > 0 && (
        <span className="objects-object-count" title={t("objects.apiCount", { count: usageCount })}>
          {usageCount}
        </span>
      )}
      <span className="objects-object-ops">
        <button className="icon-btn" title={t("objects.renameObject")} onClick={(e) => { e.stopPropagation(); onRename(); }}>
          ✎
        </button>
        <button className="icon-btn" title={t("objects.deleteObject")} onClick={(e) => { e.stopPropagation(); onDelete(); }}>
          🗑
        </button>
      </span>
    </div>
  );
}

/** JSON 值 → 对象属性列表（嵌套 object 提取为引用对象占位，由用户手动指定；数组取首元素推断元素类型） */
function jsonToObjectProps(value: unknown): ObjectProp[] {
  const out: ObjectProp[] = [];
  if (!value || typeof value !== "object" || Array.isArray(value)) return out;
  for (const [key, v] of Object.entries(value)) {
    const p: ObjectProp = {
      key,
      kind: "string",
      itemKind: "",
      refHash: "",
      description: "",
      required: false,
    };
    if (v === null) {
      p.kind = "any";
    } else if (Array.isArray(v)) {
      p.kind = "list";
      if (v.length > 0) {
        const first = v[0];
        if (first && typeof first === "object") p.itemKind = "object";
        else if (typeof first === "number") p.itemKind = "number";
        else if (typeof first === "boolean") p.itemKind = "boolean";
        else p.itemKind = "string";
      }
    } else if (typeof v === "number") {
      p.kind = "number";
    } else if (typeof v === "boolean") {
      p.kind = "boolean";
    } else if (typeof v === "object") {
      p.kind = "object";
    } else {
      p.kind = "string";
    }
    out.push(p);
  }
  return out;
}
