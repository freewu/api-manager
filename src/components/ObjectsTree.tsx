import { useEffect, useMemo, useState } from "react";
import { ObjectDef, ObjectImportResult, ObjectStore, ObjectUsageItem } from "../types";
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
  const [importOpen, setImportOpen] = useState(false);
  const [importMode, setImportMode] = useState<"json" | "ddl">("json");
  const [importName, setImportName] = useState("");
  const [importGroup, setImportGroup] = useState("");
  const [importJson, setImportJson] = useState("");
  const [importDdlText, setImportDdlText] = useState("");
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null);
  /** 对象行右键菜单（uuid） */
  const [objMenu, setObjMenu] = useState<{ x: number; y: number; uuid: string } | null>(null);
  /** 版本查看弹窗（对象 uuid） */
  const [versionModal, setVersionModal] = useState<ObjectDef | null>(null);
  // 拖拽中的对象 uuid
  const [dragUuid, setDragUuid] = useState<string | null>(null);

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

  // 导入弹窗按 ESC 关闭
  useEffect(() => {
    if (!importOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setImportOpen(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [importOpen]);

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
          style={{ paddingLeft: 6 + depth * 14 }}
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
          <span className={`caret${isOpen ? " open" : ""}`}>▸</span>
          <span className="node-icon">📁</span>
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
                void addObject(g.id);
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

  const addGroup = () => {
    const name = window.prompt(t("objects.newGroup"), "");
    if (!name || !name.trim()) return;
    const id = name.trim();
    if (store.groups.some((g) => g.id === id)) {
      onToast(t("objects.groupExists"));
      return;
    }
    void saveStore({
      groups: [...store.groups, { id, name: id.split("/").pop() || id }],
      objects: store.objects,
    });
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
    if (!window.confirm(t("objects.confirmDeleteGroup", { name: id.split("/").pop() || id }))) return;
    const groups = store.groups.filter((g) => g.id !== id && !g.id.startsWith(id + "/"));
    const objects = store.objects.map((o) =>
      o.group === id || o.group.startsWith(id + "/") ? { ...o, group: "" } : o
    );
    void saveStore({ groups, objects });
  };

  const addObject = async (groupId: string) => {
    const name = window.prompt(t("objects.newObject"), "Object");
    if (!name || !name.trim()) return;
    const now = Math.floor(Date.now() / 1000);
    const o: ObjectDef = {
      uuid: crypto.randomUUID(),
      hash: `tmp${Date.now().toString(36)}`,
      name: name.trim(),
      group: groupId,
      description: "",
      properties: [],
      createdAt: now,
      updatedAt: now,
    };
    const fresh = await saveStore({ groups: store.groups, objects: [o, ...store.objects] });
    // 新建后直接在右侧展开配置
    const created = fresh.objects.find((x) => x.name === name.trim());
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
    if (!window.confirm(t("objects.confirmDelete", { name: o.name }))) return;
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
  };

  // ===== JSON / SQL DDL 导入 =====
  const finishImport = (res: ObjectImportResult) => {
    setImportOpen(false);
    setImportName("");
    setImportGroup("");
    setImportJson("");
    setImportDdlText("");
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

  const doImport = async () => {
    try {
      if (importMode === "json") {
        if (!importName.trim()) {
          onToast(t("objects.importName"));
          return;
        }
        JSON.parse(importJson);
        const res = await onImport(importName.trim(), importGroup.trim(), importJson);
        finishImport(res);
      } else {
        if (!importDdlText.trim()) {
          onToast(t("objects.importDdlEmpty"));
          return;
        }
        const res = await onImportDdl(importGroup.trim(), importDdlText);
        finishImport(res);
      }
    } catch {
      onToast(t("objects.jsonInvalid"));
    }
  };

  return (
    <div className="history-list-side">
      <div className="history-side-toolbar">
        <div className="search-box">
          <span className="objects-search-icon">🔍</span>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t("objects.searchPlaceholder")}
            spellCheck={false}
          />
        </div>
        <button className="icon-btn" onClick={() => setImportOpen(true)} title={t("objects.importJson")}>
          ⇪
        </button>
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
              style={{ paddingLeft: 10, color: "var(--text-faint)", fontSize: 12 }}
              onClick={() => void addObject("")}
            >
              ＋ {t("objects.newObject")}
            </div>
            <div
              className="node"
              style={{ paddingLeft: 10, color: "var(--text-faint)", fontSize: 12 }}
              onClick={addGroup}
            >
              ＋ {t("objects.newGroup")}
            </div>
          </>
        )}
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
              addGroup();
            }}
          >
            📁 {t("objects.newGroup")}
          </button>
          <button
            onClick={() => {
              setCtxMenu(null);
              void addObject("");
            }}
          >
            ▦ {t("objects.newObject")}
          </button>
        </div>
      )}

      {/* JSON / SQL DDL 导入弹窗 */}
      {importOpen && (
        <div className="objects-import-mask" onClick={() => setImportOpen(false)}>
          <div className="objects-import-modal" onClick={(e) => e.stopPropagation()}>
            <div className="objects-import-title">{t("objects.importTitle")}</div>
            <div className="objects-import-modes">
              <button
                className={`objects-import-mode ${importMode === "json" ? "active" : ""}`}
                onClick={() => setImportMode("json")}
              >
                JSON
              </button>
              <button
                className={`objects-import-mode ${importMode === "ddl" ? "active" : ""}`}
                onClick={() => setImportMode("ddl")}
              >
                SQL DDL
              </button>
            </div>
            <div className="objects-import-body">
              {importMode === "json" && (
                <>
                  <label>
                    <span>{t("objects.importName")}</span>
                    <input value={importName} onChange={(e) => setImportName(e.target.value)} spellCheck={false} />
                  </label>
                  <label>
                    <span>{t("objects.importGroup")}</span>
                    <select value={importGroup} onChange={(e) => setImportGroup(e.target.value)}>
                      <option value="">{t("objects.ungrouped")}</option>
                      {store.groups.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <span>{t("objects.importJsonLabel")}</span>
                    <textarea
                      value={importJson}
                      onChange={(e) => setImportJson(e.target.value)}
                      rows={8}
                      spellCheck={false}
                      placeholder={'{\n  "id": 1,\n  "name": "alice",\n  "address": { "city": "bj" }\n}'}
                    />
                  </label>
                  <div className="objects-import-tip">{t("objects.importJsonTip")}</div>
                </>
              )}
              {importMode === "ddl" && (
                <>
                  <label>
                    <span>{t("objects.importGroup")}</span>
                    <select value={importGroup} onChange={(e) => setImportGroup(e.target.value)}>
                      <option value="">{t("objects.ungrouped")}</option>
                      {store.groups.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    <span>{t("objects.importDdlLabel")}</span>
                    <textarea
                      value={importDdlText}
                      onChange={(e) => setImportDdlText(e.target.value)}
                      rows={8}
                      spellCheck={false}
                      placeholder={"CREATE TABLE users (\n  id BIGINT PRIMARY KEY,\n  name VARCHAR(50) NOT NULL COMMENT '用户名称'\n);"}
                    />
                  </label>
                  <div className="objects-import-tip">{t("objects.importDdlTip")}</div>
                </>
              )}
            </div>
            <div className="objects-import-actions">
              <button className="btn" onClick={() => setImportOpen(false)}>
                {t("common.cancel")}
              </button>
              <button className="btn primary" onClick={() => void doImport()}>
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
      style={{ paddingLeft: 10 + depth * 14 }}
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
