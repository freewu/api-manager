import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ObjectDef,
  ObjectGroup,
  ObjectImportResult,
  ObjectProp,
  ObjectStore,
  ObjectUsageItem,
  PROP_KINDS,
} from "../types";
import { OBJECT_LANGS, generateObjectCode } from "../utils/objectCodegen";
import { objectHash } from "../utils/objectHash";
import { useT } from "../i18n";

interface Props {
  store: ObjectStore;
  usage: ObjectUsageItem[];
  onSave: (store: ObjectStore) => Promise<void>;
  onImport: (name: string, group: string, json: string) => Promise<ObjectImportResult>;
  onImportDdl: (group: string, ddl: string) => Promise<ObjectImportResult>;
  onJumpApi: (path: string) => void;
  onToast: (msg: string) => void;
}

/** 按 "/" 展开分组层级 */

export default function ObjectsView({
  store: initStore,
  usage,
  onSave,
  onImport,
  onImportDdl,
  onJumpApi,
  onToast,
}: Props) {
  const t = useT();
  const [store, setStore] = useState<ObjectStore>(() =>
    JSON.parse(JSON.stringify(initStore))
  );
  const [dirty, setDirty] = useState(false);
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [importMode, setImportMode] = useState<"json" | "ddl">("json");
  const [importName, setImportName] = useState("");
  const [importGroup, setImportGroup] = useState("");
  const [importJson, setImportJson] = useState("");
  const [importDdlText, setImportDdlText] = useState("");
  const [genLang, setGenLang] = useState("typescript");
  const [copied, setCopied] = useState(false);
  const [saving, setSaving] = useState(false);
  const [hashes, setHashes] = useState<Record<string, string>>({});
  const [search, setSearch] = useState("");

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
  const [openGroups, setOpenGroups] = useState<Set<string>>(new Set());
  const firstLoad = useRef(true);

  // 外部 store 变化（工作区切换/导入）时重置
  useEffect(() => {
    if (firstLoad.current) {
      firstLoad.current = false;
      return;
    }
    if (!dirty) {
      setStore(JSON.parse(JSON.stringify(initStore)));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initStore]);

  // 计算全部对象 hash（属性变化后实时刷新）
  useEffect(() => {
    let cancel = false;
    (async () => {
      const map: Record<string, string> = {};
      for (const o of store.objects) {
        map[o.hash] = await objectHash(o.properties || []);
      }
      if (!cancel) setHashes(map);
    })();
    return () => {
      cancel = true;
    };
  }, [store.objects]);

  const patch = useCallback((fn: (s: ObjectStore) => void) => {
    setStore((prev) => {
      const next = JSON.parse(JSON.stringify(prev)) as ObjectStore;
      fn(next);
      return next;
    });
    setDirty(true);
  }, []);

  const selected = useMemo(
    () => store.objects.find((o) => o.hash === selectedHash) || null,
    [store.objects, selectedHash]
  );

  const usageOf = useMemo(() => {
    const m: Record<string, ObjectUsageItem> = {};
    for (const u of usage) m[u.hash] = u;
    return m;
  }, [usage]);

  // ===== 分组操作 =====
  const addGroup = () => {
    const name = window.prompt(t("objects.newGroup"), "");
    if (!name || !name.trim()) return;
    if (store.groups.some((g) => g.name === name.trim())) {
      onToast(t("objects.renameGroup") + ": " + t("common.confirm"));
      return;
    }
    const id = `g${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    patch((s) => s.groups.push({ id, name: name.trim() }));
  };

  const renameGroup = (g: ObjectGroup) => {
    const name = window.prompt(t("objects.renameGroup"), g.name);
    if (!name || !name.trim() || name.trim() === g.name) return;
    patch((s) => {
      const grp = s.groups.find((x) => x.id === g.id);
      if (grp) grp.name = name.trim();
    });
  };

  const deleteGroup = (g: ObjectGroup) => {
    if (!window.confirm(t("objects.confirmDeleteGroup", { name: g.name }))) return;
    patch((s) => {
      s.groups = s.groups.filter((x) => x.id !== g.id);
      for (const o of s.objects) if (o.group === g.id) o.group = "";
    });
  };

  const addObject = () => {
    const name = window.prompt(t("objects.newObject"), "Object");
    if (!name || !name.trim()) return;
    patch((s) => {
      s.objects.unshift({
        hash: `tmp${Date.now().toString(36)}`,
        name: name.trim(),
        group: "",
        description: "",
        properties: [],
        createdAt: Math.floor(Date.now() / 1000),
        updatedAt: Math.floor(Date.now() / 1000),
      });
    });
    setSelectedHash(`tmp${Date.now().toString(36)}`);
  };

  const renameObject = (o: ObjectDef) => {
    const name = window.prompt(t("objects.renameObject"), o.name);
    if (!name || !name.trim() || name.trim() === o.name) return;
    patch((s) => {
      const obj = s.objects.find((x) => x.hash === o.hash);
      if (obj) obj.name = name.trim();
    });
  };

  const deleteObject = (o: ObjectDef) => {
    if (!window.confirm(t("objects.confirmDelete", { name: o.name }))) return;
    patch((s) => {
      s.objects = s.objects.filter((x) => x.hash !== o.hash);
      // 清空其它对象的引用
      for (const obj of s.objects) {
        for (const p of obj.properties) if (p.refHash === o.hash) p.refHash = "";
      }
    });
    if (selectedHash === o.hash) setSelectedHash(null);
  };

  // ===== 属性操作 =====
  const updateProp = (idx: number, patchP: Partial<ObjectProp>) => {
    if (!selected) return;
    patch((s) => {
      const obj = s.objects.find((x) => x.hash === selected.hash);
      if (obj && obj.properties[idx]) {
        Object.assign(obj.properties[idx], patchP);
        obj.updatedAt = Math.floor(Date.now() / 1000);
      }
    });
  };

  const addProp = () => {
    if (!selected) return;
    patch((s) => {
      const obj = s.objects.find((x) => x.hash === selected.hash);
      if (obj) {
        obj.properties.push({
          key: "",
          kind: "string",
          itemKind: "string",
          refHash: "",
          description: "",
          required: false,
        });
        obj.updatedAt = Math.floor(Date.now() / 1000);
      }
    });
  };

  const removeProp = (idx: number) => {
    if (!selected) return;
    patch((s) => {
      const obj = s.objects.find((x) => x.hash === selected.hash);
      if (obj) {
        obj.properties.splice(idx, 1);
        obj.updatedAt = Math.floor(Date.now() / 1000);
      }
    });
  };

  const save = async () => {
    setSaving(true);
    try {
      // 需求 7：相同 hash 的对象只保留第一个（自动复用），引用指向保留者
      const byReal: Map<string, ObjectDef> = new Map();
      const merged: ObjectDef[] = [];
      for (const o of store.objects) {
        const h = hashes[o.hash] || o.hash;
        if (!byReal.has(h)) {
          const copy = { ...o, hash: h } as ObjectDef;
          copy.properties = (o.properties || []).map((p) => ({ ...p }));
          byReal.set(h, copy);
          merged.push(copy);
        }
      }
      // 修正失效引用：优先按实时 hash，其次按名称
      for (const o of merged) {
        for (const p of o.properties) {
          if (!p.refHash) continue;
          if (byReal.has(p.refHash)) continue;
          const byName = merged.find((x) => x.name === p.refHash);
          p.refHash = byName ? byName.hash : "";
        }
      }
      const next: ObjectStore = { groups: store.groups, objects: merged };
      await onSave(next);
      setDirty(false);
      onToast(t("objects.saved"));
    } finally {
      setSaving(false);
    }
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
    // 选中顶层对象（复用场景 topHash 指向已有对象）
    setSelectedHash(res.topHash || (res.objects[0] && res.objects[0].hash) || null);
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

  // 分组层级渲染
  const groupTree = useMemo(() => {
    const roots: { path: string; id?: string; name: string; children: typeof roots }[] = [];
    const byPath: Record<string, (typeof roots)[number]> = {};
    const ensure = (path: string, id?: string): (typeof roots)[number] => {
      if (byPath[path]) return byPath[path];
      const node = { path, id, name: path.split("/").pop() || path, children: [] };
      byPath[path] = node;
      const parentPath = path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
      if (parentPath) {
        ensure(parentPath).children.push(node);
      } else {
        roots.push(node);
      }
      return node;
    };
    for (const g of store.groups) ensure(g.name, g.id);
    return { roots, byPath };
  }, [store.groups]);

  const objectsByGroup = useMemo(() => {
    const m: Record<string, ObjectDef[]> = { "": [] };
    for (const g of store.groups) m[g.id] = [];
    for (const o of store.objects) (m[o.group] ||= []).push(o);
    return m;
  }, [store]);

  const genCode = useMemo(() => {
    if (!selected) return "";
    const all = store.objects;
    return generateObjectCode(genLang, selected, all);
  }, [selected, store.objects, genLang]);

  const copyCode = async () => {
    try {
      await navigator.clipboard.writeText(genCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* ignore */
    }
  };

  const refCountOf = (hash: string) =>
    store.objects.reduce((n, o) => n + o.properties.filter((p) => p.refHash === hash).length, 0);

  // 左侧搜索：按对象名 / 分组名过滤（与接口管理 Sidebar 搜索一致）
  const kw = search.trim().toLowerCase();
  const filterMatch = (o: ObjectDef) => {
    if (!kw) return true;
    if (o.name.toLowerCase().includes(kw)) return true;
    const grp = store.groups.find((g) => g.id === o.group);
    if (grp && grp.name.toLowerCase().includes(kw)) return true;
    return false;
  };
  // 该分组（含后代分组）是否包含匹配对象
  const groupHasMatch = (g: (typeof groupTree.roots)[number]): boolean => {
    const grp = g.id ? store.groups.find((x) => x.id === g.id) : undefined;
    const items = grp ? objectsByGroup[grp.id] || [] : [];
    if (items.some(filterMatch)) return true;
    return g.children.some(groupHasMatch);
  };
  // 递归渲染分组树（多级分组：分组 → 子分组 → 对象），样式与 Sidebar 树一致
  const renderGroup = (g: (typeof groupTree.roots)[number], depth: number) => {
    const grp = g.id ? store.groups.find((x) => x.id === g.id) : undefined;
    const items = (grp ? objectsByGroup[grp.id] || [] : []).filter(filterMatch);
    const isOpen = openGroups.has(g.path) || !!kw;
    const nameMatch = !kw || g.name.toLowerCase().includes(kw);
    const childMatch = g.children.some(groupHasMatch);
    if (kw && !nameMatch && items.length === 0 && !childMatch) return null;
    return (
      <div key={g.path}>
        <div
          className="node objects-group-row"
          style={{ paddingLeft: 6 + depth * 14 }}
          onClick={() =>
            setOpenGroups((prev) => {
              const next = new Set(prev);
              if (next.has(g.path)) next.delete(g.path);
              else next.add(g.path);
              return next;
            })
          }
        >
          <span className={`caret${isOpen ? " open" : ""}`}>▸</span>
          <span className="node-icon">📁</span>
          <span className="node-name">{g.name}</span>
          {grp && (
            <span className="objects-group-ops">
              <button
                className="icon-btn"
                title={t("objects.renameGroup")}
                onClick={(e) => {
                  e.stopPropagation();
                  renameGroup(grp);
                }}
              >
                ✎
              </button>
              <button
                className="icon-btn"
                title={t("objects.deleteGroup")}
                onClick={(e) => {
                  e.stopPropagation();
                  deleteGroup(grp);
                }}
              >
                🗑
              </button>
            </span>
          )}
        </div>
        {isOpen && (
          <>
            {items.map((o) => (
              <ObjectRow
                key={o.hash}
                obj={o}
                depth={depth + 1}
                hash={hashes[o.hash] || o.hash}
                usageCount={usageOf[o.hash]?.apiCount ?? 0}
                selected={selectedHash === o.hash}
                onSelect={() => setSelectedHash(o.hash)}
                onRename={() => renameObject(o)}
                onDelete={() => deleteObject(o)}
              />
            ))}
            {g.children.map((c) => renderGroup(c, depth + 1))}
          </>
        )}
      </div>
    );
  };

  return (
    <div className="objects-view">
      {/* ===== 左侧：分组 + 对象列表（与接口管理 Sidebar 布局一致） ===== */}
      <div className="objects-left">
        <div className="objects-left-header">
          <span className="objects-left-title">🗂️ {t("objects.title")}</span>
          <div className="objects-left-actions">
            <button className="btn-link" onClick={addObject} title={t("objects.newObject")}>
              ＋
            </button>
            <button className="btn-link" onClick={addGroup} title={t("objects.newGroup")}>
              ＋📁
            </button>
            <button className="btn-link" onClick={() => setImportOpen(true)} title={t("objects.importJson")}>
              ⇪
            </button>
          </div>
        </div>
        <div className="objects-search-row">
          <div className="search-box">
            <span className="objects-search-icon">🔍</span>
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t("objects.searchPlaceholder")}
              spellCheck={false}
            />
          </div>
        </div>
        <div className="tree objects-list">
          {store.objects.length === 0 && <div className="objects-empty">{t("objects.empty")}</div>}
          {groupTree.roots.map((node) => renderGroup(node, 0))}
          {/* 未分组对象：直接作为顶层叶子，不展示根目录 */}
          {!kw &&
            objectsByGroup[""] &&
            objectsByGroup[""].map((o) => (
              <ObjectRow
                key={o.hash}
                obj={o}
                depth={0}
                hash={hashes[o.hash] || o.hash}
                usageCount={usageOf[o.hash]?.apiCount ?? 0}
                selected={selectedHash === o.hash}
                onSelect={() => setSelectedHash(o.hash)}
                onRename={() => renameObject(o)}
                onDelete={() => deleteObject(o)}
              />
            ))}
          {kw && (
            <>
              {store.objects.filter(filterMatch).map((o) => (
                <ObjectRow
                  key={o.hash}
                  obj={o}
                  depth={0}
                  hash={hashes[o.hash] || o.hash}
                  usageCount={usageOf[o.hash]?.apiCount ?? 0}
                  selected={selectedHash === o.hash}
                  onSelect={() => setSelectedHash(o.hash)}
                  onRename={() => renameObject(o)}
                  onDelete={() => deleteObject(o)}
                />
              ))}
            </>
          )}
        </div>
      </div>

      {/* ===== 右侧：对象配置 ===== */}
      <div className="objects-right">
        {!selected ? (
          <div className="objects-right-empty">{t("objects.empty")}</div>
        ) : (
          <div className="objects-editor">
            <div className="objects-editor-head">
              <div className="objects-editor-title">
                <input
                  className="objects-name-input"
                  value={selected.name}
                  onChange={(e) =>
                    patch((s) => {
                      const o = s.objects.find((x) => x.hash === selected.hash);
                      if (o) o.name = e.target.value;
                    })
                  }
                  spellCheck={false}
                />
                <span className="objects-hash" title={t("objects.hash")}>
                  #{hashes[selected.hash] || selected.hash}
                </span>
                <span className={`objects-dirty${dirty ? " on" : ""}`}>
                  {dirty ? t("objects.dirty") : t("objects.saved")}
                </span>
              </div>
              <div className="objects-editor-actions">
                <button className="btn" disabled={saving} onClick={() => void save()}>
                  {t("objects.save")}
                </button>
              </div>
            </div>

            <div className="objects-meta">
              <label>
                <span>{t("objects.group")}</span>
                <select
                  value={selected.group}
                  onChange={(e) =>
                    patch((s) => {
                      const o = s.objects.find((x) => x.hash === selected.hash);
                      if (o) o.group = e.target.value;
                    })
                  }
                >
                  <option value="">{t("objects.ungrouped")}</option>
                  {store.groups.map((g) => (
                    <option key={g.id} value={g.id}>
                      {g.name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="objects-desc-field">
                <span>{t("objects.description")}</span>
                <input
                  value={selected.description}
                  onChange={(e) =>
                    patch((s) => {
                      const o = s.objects.find((x) => x.hash === selected.hash);
                      if (o) o.description = e.target.value;
                    })
                  }
                  spellCheck={false}
                />
              </label>
            </div>

            {/* 属性配置：表格样式与接口文档 tab 响应一致 */}
            <div className="objects-props">
              <div className="objects-section-title">
                {t("objects.properties")}
                <button className="btn-link" onClick={addProp}>
                  ＋{t("objects.addProp")}
                </button>
              </div>
              <table className="kv-table doc-params-table objects-props-table">
                <thead>
                  <tr>
                    <th>{t("objects.propKey")}</th>
                    <th style={{ width: 110 }}>{t("kv.type")}</th>
                    <th style={{ width: 130 }}>{t("objects.itemKind")}</th>
                    <th style={{ width: 170 }}>{t("objects.refObject")}</th>
                    <th style={{ width: 56 }}>{t("objects.propRequired")}</th>
                    <th>{t("objects.propDesc")}</th>
                    <th style={{ width: 30 }} />
                  </tr>
                </thead>
                <tbody>
                  {selected.properties.map((p, i) => {
                    const isObjRef = p.kind === "object" || (p.kind === "list" && p.itemKind === "object");
                    return (
                      <tr key={i}>
                        <td>
                          <input
                            className="doc-key-input"
                            value={p.key}
                            placeholder="name"
                            onChange={(e) => updateProp(i, { key: e.target.value })}
                            spellCheck={false}
                          />
                        </td>
                        <td>
                          <select
                            className="doc-type-select"
                            value={p.kind}
                            onChange={(e) => updateProp(i, { kind: e.target.value })}
                          >
                            {PROP_KINDS.map((k) => (
                              <option key={k} value={k}>
                                {k}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>
                          {p.kind === "list" ? (
                            <select
                              className="doc-type-select"
                              value={p.itemKind}
                              onChange={(e) => updateProp(i, { itemKind: e.target.value })}
                            >
                              {["string", "number", "boolean", "object", "any"].map((k) => (
                                <option key={k} value={k}>
                                  {k}
                                </option>
                              ))}
                            </select>
                          ) : (
                            <span className="doc-value-manual">—</span>
                          )}
                        </td>
                        <td>
                          {isObjRef ? (
                            <div className="objects-ref-cell">
                              <select
                                className="doc-object-select"
                                value={p.refHash}
                                onChange={(e) => updateProp(i, { refHash: e.target.value })}
                              >
                                <option value="">—</option>
                                {store.objects
                                  .filter((o) => o.hash !== selected.hash)
                                  .map((o) => (
                                    <option key={o.hash} value={hashes[o.hash] || o.hash}>
                                      {o.name}
                                    </option>
                                  ))}
                              </select>
                              {p.refHash && (
                                <button
                                  className="icon-btn objects-ref-jump"
                                  title={t("objects.jump")}
                                  onClick={() => setSelectedHash(p.refHash)}
                                >
                                  →
                                </button>
                              )}
                            </div>
                          ) : (
                            <span className="doc-value-manual">—</span>
                          )}
                        </td>
                        <td>
                          <div className="kv-check">
                            <input
                              type="checkbox"
                              checked={p.required}
                              onChange={(e) => updateProp(i, { required: e.target.checked })}
                            />
                          </div>
                        </td>
                        <td>
                          <input
                            value={p.description}
                            placeholder={t("objects.propDesc")}
                            onChange={(e) => updateProp(i, { description: e.target.value })}
                            spellCheck={false}
                          />
                        </td>
                        <td>
                          <button
                            className="icon-btn"
                            title={t("common.delete")}
                            onClick={() => removeProp(i)}
                          >
                            ✕
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                  {selected.properties.length === 0 && (
                    <tr>
                      <td colSpan={7} className="doc-empty">
                        {t("objects.addProp")}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>

            {/* 引用统计 */}
            <div className="objects-usage">
              <div className="objects-section-title">{t("objects.usage")}</div>
              {(() => {
                const u = usageOf[selected.hash];
                const refBy = refCountOf(selected.hash);
                return (
                  <div className="objects-usage-body">
                    <div className="objects-usage-item">
                      <span className="objects-usage-label">{t("objects.apiCount", { count: u?.apiCount ?? 0 })}</span>
                      {u && u.apis.length > 0 && (
                        <div className="objects-usage-apis">
                          {u.apis.map((a, i) => (
                            <button
                              key={i}
                              className="objects-usage-api"
                              onClick={() => onJumpApi(a.path)}
                              title={a.path}
                            >
                              <span className={`method-${a.method.toLowerCase()}`}>{a.method}</span>
                              {a.name}
                              <span className="objects-usage-jump">{t("objects.jump")} →</span>
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                    <div className="objects-usage-item">
                      <span className="objects-usage-label">
                        {refBy > 0
                          ? t("objects.referencedByCount", { count: refBy })
                          : t("objects.noUsage")}
                      </span>
                    </div>
                    {selected.updatedAt > 0 && (
                      <div className="objects-usage-item">
                        <span className="objects-usage-label">
                          {t("objects.createdAt")}: {new Date(selected.createdAt * 1000).toLocaleString()}
                          {"  ·  "}
                          {t("objects.updatedAt")}: {new Date(selected.updatedAt * 1000).toLocaleString()}
                        </span>
                      </div>
                    )}
                  </div>
                );
              })()}
            </div>

            {/* 代码生成 */}
            <div className="objects-codegen">
              <div className="objects-section-title">
                {t("objects.codegen")}
                <span className="objects-codegen-tip">{t("objects.codegenTip")}</span>
              </div>
              <div className="objects-codegen-bar">
                <select value={genLang} onChange={(e) => setGenLang(e.target.value)}>
                  {OBJECT_LANGS.map((l) => (
                    <option key={l.value} value={l.value}>
                      {l.label}
                    </option>
                  ))}
                </select>
                <button className="btn" onClick={() => void copyCode()}>
                  {copied ? t("objects.copied") : t("objects.copyCode")}
                </button>
              </div>
              <pre className="objects-codegen-pre">{genCode || "//"}</pre>
            </div>
          </div>
        )}
      </div>

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
                      rows={10}
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
                      rows={10}
                      spellCheck={false}
                      placeholder={'CREATE TABLE users (\n  id BIGINT PRIMARY KEY,\n  name VARCHAR(50) NOT NULL COMMENT \'用户名称\',\n  age INT\n);'}
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
    </div>
  );
}

function ObjectRow({
  obj,
  depth,
  hash,
  usageCount,
  selected,
  onSelect,
  onRename,
  onDelete,
}: {
  obj: ObjectDef;
  depth: number;
  hash: string;
  usageCount: number;
  selected: boolean;
  onSelect: () => void;
  onRename: () => void;
  onDelete: () => void;
}) {
  const t = useT();
  return (
    <div
      className={`node objects-object-row${selected ? " selected" : ""}`}
      style={{ paddingLeft: 10 + depth * 14 }}
      onClick={onSelect}
    >
      <span className="node-icon objects-object-icon">▦</span>
      <span className="node-name objects-object-name">{obj.name}</span>
      <span className="objects-object-hash">#{hash}</span>
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
