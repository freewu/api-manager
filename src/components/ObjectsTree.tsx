import { useEffect, useMemo, useState } from "react";
import { ObjectDef, ObjectImportResult, ObjectProp, ObjectStore, ObjectUsageItem } from "../types";

/** 对象名称校验：字母开头，仅允许字母和数字（不允许空格） */
import { useT } from "../i18n";
import { ObjectVersionModal } from "./ObjectVersionModal";
import GroupStatsModal from "./GroupStatsModal";
import GenDataModal from "./GenDataModal";
import { openPath, GenDataResult } from "../commands";

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
  /** 右侧空状态请求：新增对象（每次 +1 触发打开新建弹窗） */
  newReq: number;
  /** 右侧空状态请求：导入对象（打开新建弹窗并聚焦 JSON 输入） */
  importReq: number;
}

/** 对象管理：左侧树形目录（分组 = 目录，多级；不显示根目录） */
export default function ObjectsTree({
  store,
  usage,
  onSave,
  onImport: _onImport,
  onImportDdl: _onImportDdl,
  onToast,
  selectedUuid,
  onSelectObject,
  newReq,
  importReq,
}: Props) {
  const t = useT();
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  /** 删除确认弹窗（对象 / 分组） */
  const [confirmDel, setConfirmDel] = useState<{ title: string; message: string; onConfirm: () => void } | null>(null);
  /** 树内联编辑（双击行触发：对象改显示名；右键重命名：对象改文件名；分组改分组名） */
  const [inlineEdit, setInlineEdit] = useState<
    | { kind: "object"; uuid: string; value: string }
    | { kind: "objname"; uuid: string; value: string }
    | { kind: "group"; id: string; value: string }
    | null
  >(null);
  /** 新增对象弹窗（名称 + JSON，JSON 可为空） */
  const [newOpen, setNewOpen] = useState(false);
  const [newDisplayName, setNewDisplayName] = useState("");
  const [newJson, setNewJson] = useState("");
  const [newGroup, setNewGroup] = useState("");
  /** 右侧空状态请求：打开新增对象弹窗 / 导入（聚焦 JSON） */
  const [focusJson, setFocusJson] = useState(false);
  useEffect(() => {
    if (newReq > 0) {
      setFocusJson(false);
      openNewObject("");
    }
  }, [newReq]);
  useEffect(() => {
    if (importReq > 0) {
      setFocusJson(true);
      openNewObject("");
    }
  }, [importReq]);
  /** 新建分组弹窗 */
  const [groupOpen, setGroupOpen] = useState(false);
  const [groupName, setGroupName] = useState("");
  /** 父分组 id（空 = 顶层） */
  const [groupParent, setGroupParent] = useState("");
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number } | null>(null);
  /** 数据生成弹窗对象（null = 未打开） */
  const [genDataObj, setGenDataObj] = useState<ObjectDef | null>(null);
  /** 数据生成完成提示（右下角，点击打开目录） */
  const [genDone, setGenDone] = useState<GenDataResult | null>(null);
  /** 对象行右键菜单（uuid） */
  const [objMenu, setObjMenu] = useState<{ x: number; y: number; uuid: string } | null>(null);
  const [groupMenu, setGroupMenu] = useState<{ x: number; y: number; id: string } | null>(null);
  /** 分组统计弹窗 */
  const [groupStats, setGroupStats] = useState<{ id: string; name: string } | null>(null);
  /** 版本查看弹窗（对象 uuid） */
  const [versionModal, setVersionModal] = useState<ObjectDef | null>(null);
  // 拖拽中的对象 uuid
  const [dragUuid, setDragUuid] = useState<string | null>(null);

  // 右键菜单：点击任意处 / Esc 时关闭
  useEffect(() => {
    if (!ctxMenu && !objMenu && !groupMenu) return;
    const close = () => {
      setCtxMenu(null);
      setObjMenu(null);
      setGroupMenu(null);
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
  }, [ctxMenu, objMenu, groupMenu]);

  const usageOf = useMemo(() => {
    const m: Record<string, ObjectUsageItem> = {};
    for (const u of usage) m[u.hash] = u;
    return m;
  }, [usage]);

  /** 对象是否展示为废弃：自身已废弃，或所属分组（含任意祖先分组）已废弃 */
  const isObjDeprecated = (o: ObjectDef): boolean =>
    o.deprecated ||
    store.groups.some((g) => g.deprecated && (o.group === g.id || o.group.startsWith(g.id + "/")));

  const kw = search.trim().toLowerCase();
  const filterMatch = (o: ObjectDef) => {
    if (!kw) return true;
    if (o.name.toLowerCase().includes(kw)) return true;
    if ((o.displayName || "").toLowerCase().includes(kw)) return true;
    const grp = store.groups.find((g) => g.id === o.group);
    if (grp && grp.name.toLowerCase().includes(kw)) return true;
    return false;
  };

  // 分组树：id 即路径（"父/子"），不显示根目录
  interface GNode {
    id: string;
    name: string;
    deprecated: boolean;
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
        deprecated: store.groups.find((g) => g.id === id)?.deprecated ?? false,
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
          className={`node objects-group-row${g.deprecated ? " deprecated" : ""}`}
          style={{ paddingLeft: 6 + depth * 14 }}
          onClick={() =>
            setOpenGroups((prev) => {
              const next = new Set(prev);
              if (next.has(g.id)) next.delete(g.id);
              else next.add(g.id);
              return next;
            })
          }
          onContextMenu={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setGroupMenu({ x: e.clientX, y: e.clientY, id: g.id });
          }}
          onDoubleClick={(e) => {
            e.stopPropagation();
            setInlineEdit({ kind: "group", id: g.id, value: g.name });
          }}
          title={t("objects.dblclickEdit")}
          onDragOver={(e) => {
            if (dragUuid) e.preventDefault();
          }}
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (dragUuid) moveObject(dragUuid, g.id);
          }}
        >
          <span className={`caret${isOpen ? " open" : ""}`}>▶</span>
          <span className="node-icon">📁</span>
          {inlineEdit?.kind === "group" && inlineEdit.id === g.id ? (
            <input
              className="objects-inline-input"
              value={inlineEdit.value}
              autoFocus
              onChange={(e) => setInlineEdit({ kind: "group", id: g.id, value: e.target.value })}
              onClick={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                e.stopPropagation();
                if (e.key === "Enter") commitInlineEdit();
                else if (e.key === "Escape") setInlineEdit(null);
              }}
              onBlur={commitInlineEdit}
            />
          ) : (
            <span className="node-name">{g.name}</span>
          )}
          {g.deprecated && <span className="objects-deprecated-badge">已废弃</span>}
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
                setInlineEdit({ kind: "group", id: g.id, value: g.name });
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
                deprecated={isObjDeprecated(o)}
                onSelect={() => onSelectObject(o.uuid)}
                onStartEdit={() => setInlineEdit({ kind: "object", uuid: o.uuid, value: o.displayName || "" })}
                editActive={inlineEdit?.kind === "object" && inlineEdit.uuid === o.uuid}
                editValue={inlineEdit?.kind === "object" && inlineEdit.uuid === o.uuid ? inlineEdit.value : ""}
                onEditChange={(v) => setInlineEdit({ kind: "object", uuid: o.uuid, value: v })}
                onCommitEdit={commitInlineEdit}
                onCancelEdit={() => setInlineEdit(null)}
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
            {/* 分组末尾：新建子分组 */}
            <div
              className="node objects-new-subgroup"
              style={{ paddingLeft: 6 + (depth + 1) * 14, color: "var(--text-faint)", fontSize: 12 }}
              onClick={(e) => {
                e.stopPropagation();
                openNewGroup(g.id);
              }}
            >
              ＋ {t("objects.newGroup")}
            </div>
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

  const openNewGroup = (parentId = "") => {
    setGroupParent(parentId);
    setGroupName("");
    setGroupOpen(true);
  };

  const doNewGroup = async () => {
    const name = groupName.trim();
    if (!name) {
      onToast(t("objects.newGroupNameEmpty"));
      return;
    }
    // 父分组下新建：id = 父id/子名（name 取最后一段）
    const id = groupParent ? `${groupParent}/${name}` : name;
    if (store.groups.some((g) => g.id === id)) {
      onToast(t("objects.groupExists"));
      return;
    }
    setGroupOpen(false);
    await saveStore({
      groups: [...store.groups, { id, name: id.split("/").pop() || id, deprecated: false }],
      objects: store.objects,
    });
    // 新建后自动展开该分组及其父级
    setOpenGroups((prev) => {
      const next = new Set(prev);
      const parts = id.split("/");
      let acc = "";
      for (const p of parts) {
        acc = acc ? `${acc}/${p}` : p;
        next.add(acc);
      }
      return next;
    });
  };

  // 展开全部 / 收起全部（按钮已移除，保留默认展开第一层逻辑）

  /** 文件导入已移除（.json / .sql） */

  /** 分组改名（id 即目录路径，同步子分组与组内对象） */
  const renameGroupImpl = (id: string, newName: string) => {
    if (!newName.trim() || newName.trim() === id.split("/").pop()) return;
    const p = id.lastIndexOf("/");
    const newId = p > 0 ? `${id.slice(0, p)}/${newName.trim()}` : newName.trim();
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

  /** 树内联编辑提交：对象 → displayName；对象重命名 → name（文件名）；分组 → 分组名 */
  const commitInlineEdit = () => {
    const e = inlineEdit;
    if (!e) return;
    setInlineEdit(null);
    if (e.kind === "object") {
      const v = e.value.trim();
      void saveStore({
        groups: store.groups,
        objects: store.objects.map((o) =>
          o.uuid === e.uuid ? (v ? { ...o, displayName: v } : { ...o, displayName: undefined }) : o
        ),
      });
    } else if (e.kind === "objname") {
      // 对象文件名不做约束（DDL 导入可能用中文表注释），仅要求非空
      const v = e.value.trim();
      if (!v) {
        onToast(t("objects.nameEmpty"));
        return;
      }
      if (v === store.objects.find((o) => o.uuid === e.uuid)?.name) return;
      void saveStore({
        groups: store.groups,
        objects: store.objects.map((o) => (o.uuid === e.uuid ? { ...o, name: v } : o)),
      });
    } else {
      renameGroupImpl(e.id, e.value);
    }
  };

  /** 切换分组/对象的已废弃标记 */
  const toggleDeprecated = (target: "group" | "object", id: string) => {
    if (target === "group") {
      void saveStore({
        groups: store.groups.map((g) => (g.id === id ? { ...g, deprecated: !g.deprecated } : g)),
        objects: store.objects,
      });
    } else {
      void saveStore({
        groups: store.groups,
        objects: store.objects.map((o) => (o.uuid === id ? { ...o, deprecated: !o.deprecated } : o)),
      });
    }
  };

  /** 复制分组（含全部子分组与对象）：对象 uuid 重新生成，其余字段保持一致 */
  const duplicateGroup = (gid: string) => {
    const g = store.groups.find((x) => x.id === gid);
    if (!g) return;
    const copyId = `${gid}Copy`;
    if (store.groups.some((x) => x.id === copyId)) {
      onToast(t("objects.groupExists"));
      return;
    }
    const inSub = (id: string) => id === gid || id.startsWith(`${gid}/`);
    const now = Math.floor(Date.now() / 1000);
    // 分组：根分组 id 加 Copy 后缀，子分组父路径同步替换；name 沿用原名（根分组取新 id 最后一段）
    const newGroups = store.groups
      .filter((x) => inSub(x.id))
      .map((x) => {
        const newId = copyId + x.id.slice(gid.length);
        return { ...x, id: newId, name: newId.split("/").pop() || newId };
      });
    const newObjects = store.objects
      .filter((o) => inSub(o.group))
      .map((o) => ({
        ...o,
        uuid: crypto.randomUUID(),
        group: copyId + o.group.slice(gid.length),
        createdAt: now,
        updatedAt: now,
      }));
    void saveStore({
      groups: [...store.groups, ...newGroups],
      objects: [...store.objects, ...newObjects],
    });
    onToast(t("objects.copyGroupDone", { name: g.name }));
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
    setNewDisplayName("");
    setNewJson("");
    setNewOpen(true);
  };

  /** 弹窗确认：JSON 可选（空 = 创建空对象）；对象名称自动生成（即文件名 <名称>.obj.json） */
  const doNewObject = async () => {
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
    const name = `obj${Date.now().toString(36)}`;
    const o: ObjectDef = {
      uuid: crypto.randomUUID(),
      hash: `tmp${Date.now().toString(36)}`,
      name,
      displayName: newDisplayName.trim() || undefined,
      group: newGroup,
      deprecated: false,
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

  /** 复制对象：同分组深拷贝副本（新 uuid，英文名加 Copy 后缀） */
  const duplicateObject = (o: ObjectDef) => {
    const copy: ObjectDef = {
      ...JSON.parse(JSON.stringify(o)),
      uuid: crypto.randomUUID(),
      name: `${o.name}Copy`,
      displayName: o.displayName ? `${o.displayName}${t("objects.copySuffix")}` : undefined,
      createdAt: Math.floor(Date.now() / 1000),
      updatedAt: Math.floor(Date.now() / 1000),
    };
    void saveStore({ groups: store.groups, objects: [copy, ...store.objects] });
  };

  /** 删除对象 */
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
        onDragOver={(e) => {
          if (dragUuid) e.preventDefault();
        }}
        onDrop={(e) => {
          e.preventDefault();
          if (dragUuid) moveObject(dragUuid, "");
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
              deprecated={isObjDeprecated(o)}
              onSelect={() => onSelectObject(o.uuid)}
              onStartEdit={() => setInlineEdit({ kind: "object", uuid: o.uuid, value: o.displayName || "" })}
              editActive={(inlineEdit?.kind === "object" || inlineEdit?.kind === "objname") && inlineEdit.uuid === o.uuid}
              editValue={
                (inlineEdit?.kind === "object" || inlineEdit?.kind === "objname") && inlineEdit.uuid === o.uuid
                  ? inlineEdit.value
                  : ""
              }
              onEditChange={(v) =>
                setInlineEdit({
                  kind: inlineEdit?.kind === "objname" ? "objname" : "object",
                  uuid: o.uuid,
                  value: v,
                })
              }
              onCommitEdit={commitInlineEdit}
              onCancelEdit={() => setInlineEdit(null)}
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
              deprecated={isObjDeprecated(o)}
              onSelect={() => onSelectObject(o.uuid)}
              onStartEdit={() => setInlineEdit({ kind: "object", uuid: o.uuid, value: o.displayName || "" })}
              editActive={(inlineEdit?.kind === "object" || inlineEdit?.kind === "objname") && inlineEdit.uuid === o.uuid}
              editValue={
                (inlineEdit?.kind === "object" || inlineEdit?.kind === "objname") && inlineEdit.uuid === o.uuid
                  ? inlineEdit.value
                  : ""
              }
              onEditChange={(v) =>
                setInlineEdit({
                  kind: inlineEdit?.kind === "objname" ? "objname" : "object",
                  uuid: o.uuid,
                  value: v,
                })
              }
              onCommitEdit={commitInlineEdit}
              onCancelEdit={() => setInlineEdit(null)}
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
              onClick={() => openNewGroup()}
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
              if (o) setInlineEdit({ kind: "objname", uuid: o.uuid, value: o.name });
            }}
          >
            ✎ {t("objects.renameObject")}
          </button>
          <button
            onClick={() => {
              const o = store.objects.find((x) => x.uuid === objMenu.uuid);
              setObjMenu(null);
              if (o) duplicateObject(o);
            }}
          >
            📋 {t("objects.copy")}
          </button>
          <button
            onClick={() => {
              const o = store.objects.find((x) => x.uuid === objMenu.uuid);
              setObjMenu(null);
              if (o) setGenDataObj(o);
            }}
          >
            ⚙ {t("objects.genData")}
          </button>
          <button
            onClick={() => {
              setObjMenu(null);
              toggleDeprecated("object", objMenu.uuid);
            }}
          >
            🚫 {store.objects.find((x) => x.uuid === objMenu.uuid)?.deprecated
              ? t("objects.undeprecate")
              : t("objects.deprecate")}
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

      {/* 分组行右键菜单：废弃切换 / 删除 */}
      {groupMenu && (
        <div className="node-ctx-menu" style={{ left: groupMenu.x, top: groupMenu.y }}>
          <button
            onClick={() => {
              setGroupMenu(null);
              duplicateGroup(groupMenu.id);
            }}
          >
            📋 {t("objects.copyGroup")}
          </button>
          <button
            onClick={() => {
              const g = store.groups.find((x) => x.id === groupMenu.id);
              setGroupMenu(null);
              if (g) setGroupStats({ id: g.id, name: g.name });
            }}
          >
            📊 {t("objects.groupStats")}
          </button>
          <button
            onClick={() => {
              setGroupMenu(null);
              toggleDeprecated("group", groupMenu.id);
            }}
          >
            🚫 {store.groups.find((g) => g.id === groupMenu.id)?.deprecated
              ? t("objects.undeprecate")
              : t("objects.deprecate")}
          </button>
          <button
            className="danger"
            onClick={() => {
              const g = store.groups.find((x) => x.id === groupMenu.id);
              setGroupMenu(null);
              if (g) deleteGroup(g.id);
            }}
          >
            🗑 {t("objects.deleteGroup")}
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
          <button
            onClick={() => {
              setCtxMenu(null);
              setGroupStats({ id: "", name: t("objects.statAll") });
            }}
          >
            📊 {t("objects.statAll")}
          </button>
        </div>
      )}

      {/* 分组统计弹窗 */}
      {groupStats && (
        <GroupStatsModal
          groupName={groupStats.name}
          groups={
            groupStats.id
              ? store.groups.filter((g) => g.id === groupStats.id || g.id.startsWith(groupStats.id + "/"))
              : store.groups
          }
          objects={
            groupStats.id
              ? store.objects.filter(
                  (o) => o.group === groupStats.id || o.group.startsWith(groupStats.id + "/")
                )
              : store.objects
          }
          isDeprecated={isObjDeprecated}
          onClose={() => setGroupStats(null)}
        />
      )}

      {/* 数据生成弹窗 */}
      {genDataObj && (
        <GenDataModal
          obj={genDataObj}
          onClose={() => setGenDataObj(null)}
          onDone={(r) => setGenDone(r)}
          t={t}
        />
      )}

      {/* 生成完成右下角提示：点击打开文件所在目录 */}
      {genDone && (
        <div
          className="gen-done-toast"
          onClick={() => {
            void openPath(genDone.dir);
            setGenDone(null);
          }}
        >
          <div className="gen-done-title">✅ {t("objects.genDataDone")}</div>
          <div className="gen-done-file">
            {genDone.file}（{genDone.count} 条，{t("objects.genDataElapsed", { s: (genDone.elapsed_ms / 1000).toFixed(1) })}）
          </div>
          <div className="gen-done-open">📂 {t("objects.genDataOpenDir")}</div>
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
                <span>{t("objects.displayName")}</span>
                <input
                  value={newDisplayName}
                  onChange={(e) => setNewDisplayName(e.target.value)}
                  autoFocus={!(focusJson && newOpen)}
                  spellCheck={false}
                />
              </label>
              <label>
                <span>{t("objects.newObjectJson")}</span>
                <textarea
                  value={newJson}
                  onChange={(e) => setNewJson(e.target.value)}
                  rows={7}
                  autoFocus={focusJson && newOpen}
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
  deprecated,
  onSelect,
  onStartEdit,
  editActive,
  editValue,
  onEditChange,
  onContextMenu,
  onDragStart,
  onDragEnd,
  onCommitEdit,
  onCancelEdit,
}: {
  obj: ObjectDef;
  depth: number;
  usageCount: number;
  selected: boolean;
  /** 展示为废弃（自身或所属分组废弃） */
  deprecated: boolean;
  onSelect: () => void;
  /** 双击进入内联编辑（显示名称） */
  onStartEdit: () => void;
  editActive: boolean;
  editValue: string;
  onEditChange: (v: string) => void;
  onContextMenu: (e: React.MouseEvent, uuid: string) => void;
  onDragStart: () => void;
  onDragEnd: () => void;
  onCommitEdit: () => void;
  onCancelEdit: () => void;
}) {
  const t = useT();
  return (
    <div
      className={`node objects-object-row${selected ? " selected" : ""}${deprecated ? " deprecated" : ""}`}
      style={{ paddingLeft: 6 + depth * 14 }}
      onClick={onSelect}
      onDoubleClick={(e) => {
        e.stopPropagation();
        onStartEdit();
      }}
      onContextMenu={(e) => onContextMenu(e, obj.uuid)}
      title={t("objects.dblclickEdit")}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("text/plain", obj.uuid);
        e.dataTransfer.effectAllowed = "move";
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      onDragOver={(e) => e.stopPropagation()}
      onDrop={(e) => e.stopPropagation()}
    >
      <span className="node-icon objects-object-icon">▦</span>
      {editActive ? (
        <input
          className="objects-inline-input"
          value={editValue}
          autoFocus
          onChange={(e) => onEditChange(e.target.value)}
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Enter") onCommitEdit();
            else if (e.key === "Escape") onCancelEdit();
          }}
          onBlur={onCommitEdit}
        />
      ) : (
        <span className="node-name objects-object-name">
          {obj.displayName || obj.name}
          {obj.displayName && <span className="objects-object-ename">{obj.name}</span>}
          {deprecated && <span className="objects-deprecated-badge">已废弃</span>}
        </span>
      )}
      {usageCount > 0 && (
        <span className="objects-object-count" title={t("objects.apiCount", { count: usageCount })}>
          {usageCount}
        </span>
      )}
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
      mock: "",
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
