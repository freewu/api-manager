import { useEffect, useMemo, useState } from "react";
import { useT } from "../i18n";
import { saveObjectVersion } from "../commands";
import {
  ObjectDef,
  ObjectImportResult,
  ObjectProp,
  ObjectStore,
  ObjectUsageItem,
  PROP_KINDS,
} from "../types";

interface Props {
  store: ObjectStore;
  usage: ObjectUsageItem[];
  onSave: (store: ObjectStore) => Promise<ObjectStore>;
  onImport: (name: string, group: string, json: string) => Promise<ObjectImportResult>;
  onImportDdl: (group: string, ddl: string) => Promise<ObjectImportResult>;
  onJumpApi: (path: string) => void;
  onToast: (msg: string) => void;
  /** 当前选中对象 uuid（null = 未选中） */
  selectedUuid: string | null;
  onSelectObject: (uuid: string | null) => void;
}

const ITEM_KINDS = ["string", "number", "boolean", "object", "any"];

function emptyProp(): ObjectProp {
  return { key: "", kind: "string", itemKind: "", refHash: "", description: "", required: false };
}

/** 对象管理：右侧对象配置（类似接口文档管理的响应编辑，kv 表格风格） */
export default function ObjectsView({
  store,
  usage,
  onSave,
  onImport: _onImport,
  onImportDdl: _onImportDdl,
  onJumpApi: _onJumpApi,
  onToast,
  selectedUuid,
  onSelectObject,
}: Props) {
  const t = useT();
  const selected = useMemo(
    () => store.objects.find((o) => o.uuid === selectedUuid) || null,
    [store.objects, selectedUuid]
  );

  // 编辑草稿：选中对象变化时重置
  const [draft, setDraft] = useState<ObjectDef | null>(null);
  useEffect(() => {
    setDraft(selected ? JSON.parse(JSON.stringify(selected)) : null);
  }, [selectedUuid, store.objects]);

  const dirty = useMemo(() => {
    if (!selected || !draft) return false;
    return JSON.stringify(selected) !== JSON.stringify(draft);
  }, [selected, draft]);

  const usageOf = useMemo(() => {
    const m: Record<string, ObjectUsageItem> = {};
    for (const u of usage) m[u.hash] = u;
    return m;
  }, [usage]);

  if (!selected || !draft) {
    return (
      <div className="objects-blank">
        <span>{t("objects.selectHint")}</span>
      </div>
    );
  }

  const patch = (fn: (d: ObjectDef) => void) => {
    const next = JSON.parse(JSON.stringify(draft)) as ObjectDef;
    fn(next);
    setDraft(next);
  };

  const save = async () => {
    if (!dirty || !draft) return;
    // 保存前自动记录对象版本快照（.object_version/<uuid>/<n>.json）
    const snapshot = JSON.parse(JSON.stringify(draft)) as ObjectDef;
    if (!snapshot.uuid) snapshot.uuid = crypto.randomUUID();
    try {
      await saveObjectVersion(snapshot.uuid, snapshot);
    } catch {
      // 版本保存失败不阻断主保存
    }
    const next = {
      groups: store.groups,
      objects: store.objects.map((o) => (o.uuid === selectedUuid ? snapshot : o)),
    };
    try {
      const fresh = await onSave(next);
      onToast(t("objects.saved"));
      // 按稳定 uuid 重新定位选中项
      const updated = fresh.objects.find((o) => o.uuid === snapshot.uuid);
      if (updated) onSelectObject(updated.uuid);
    } catch {
      onToast(t("toast.saveFailed"));
    }
  };

  const addProp = () => {
    const next = JSON.parse(JSON.stringify(draft)) as ObjectDef;
    next.properties.push(emptyProp());
    setDraft(next);
  };
  const removeProp = (i: number) => {
    const next = JSON.parse(JSON.stringify(draft)) as ObjectDef;
    next.properties.splice(i, 1);
    setDraft(next);
  };
  const setProp = (i: number, p: ObjectProp) => {
    const next = JSON.parse(JSON.stringify(draft)) as ObjectDef;
    next.properties[i] = p;
    setDraft(next);
  };

  const usageItem = usageOf[selected.hash];

  return (
    <div className="history-view-content">
      <div className="history-detail objects-detail">
        {/* 头部：名称 / 分组 / hash / 时间 / 保存 */}
        <div className="objects-detail-head">
          <input
            className="objects-name-input"
            value={draft.name}
            onChange={(e) => patch((d) => void (d.name = e.target.value))}
            spellCheck={false}
          />
          <select
            className="objects-group-select"
            value={draft.group}
            onChange={(e) => patch((d) => void (d.group = e.target.value))}
          >
            <option value="">{t("objects.ungrouped")}</option>
            {store.groups.map((g) => (
              <option key={g.id} value={g.id}>
                {g.name}
              </option>
            ))}
          </select>
          <span className="objects-hash" title={selected.hash}>
            {selected.hash.slice(0, 12)}
          </span>
          <span className="objects-meta">
            {t("objects.createdAt")} {fmt(selected.createdAt)}
            {"  "}
            {t("objects.updatedAt")} {fmt(selected.updatedAt)}
          </span>
          {usageItem && usageItem.apiCount > 0 && (
            <span className="objects-usage" title={usageItem.apis.map((a) => `${a.method} ${a.path}`).join("\n")}>
              {t("objects.apiCount", { count: usageItem.apiCount })}
            </span>
          )}
          <button className="btn primary objects-save-btn" disabled={!dirty} onClick={() => void save()}>
            {t("objects.save")}
          </button>
        </div>

        {/* 描述 */}
        <div className="objects-desc-field">
          <input
            value={draft.description}
            onChange={(e) => patch((d) => void (d.description = e.target.value))}
            placeholder={t("objects.description")}
            spellCheck={false}
          />
        </div>

        {/* 属性表（kv 表格风格） */}
        <div className="objects-section-title">
          {t("objects.props")}
          <button className="btn-sm" onClick={addProp}>
            ＋ {t("objects.addProp")}
          </button>
        </div>
        <div className="objects-props-wrap">
          <table className="doc-params-table">
            <thead>
              <tr>
                <th style={{ width: "22%" }}>{t("objects.propKey")}</th>
                <th style={{ width: "11%" }}>{t("objects.propType")}</th>
                <th style={{ width: "12%" }}>{t("objects.propItemType")}</th>
                <th style={{ width: "15%" }}>{t("objects.propRef")}</th>
                <th style={{ width: "6%" }}>{t("objects.propRequired")}</th>
                <th style={{ width: "24%" }}>{t("objects.propDesc")}</th>
                <th style={{ width: "10%" }}>{t("objects.propOps")}</th>
              </tr>
            </thead>
            <tbody>
              {draft.properties.map((p, i) => (
                <tr key={i}>
                  <td>
                    <input
                      className="doc-key-input objects-prop-input"
                      value={p.key}
                      onChange={(e) => setProp(i, { ...p, key: e.target.value })}
                      placeholder="field"
                      spellCheck={false}
                    />
                  </td>
                  <td>
                    <select
                      className="doc-type-select"
                      value={p.kind}
                      onChange={(e) =>
                        setProp(i, {
                          ...p,
                          kind: e.target.value,
                          itemKind: e.target.value === "list" ? "string" : "",
                          refHash: ["object", "list"].includes(e.target.value) ? p.refHash : "",
                        })
                      }
                    >
                      {PROP_KINDS.map((k) => (
                        <option key={k} value={k}>
                          {k}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    {p.kind === "list" && (
                      <select
                        className="doc-type-select"
                        value={p.itemKind}
                        onChange={(e) =>
                          setProp(i, {
                            ...p,
                            itemKind: e.target.value,
                            refHash: e.target.value === "object" ? p.refHash : "",
                          })
                        }
                      >
                        {ITEM_KINDS.map((k) => (
                          <option key={k} value={k}>
                            {k}
                          </option>
                        ))}
                      </select>
                    )}
                  </td>
                  <td className="objects-ref-cell">
                    {(p.kind === "object" || (p.kind === "list" && p.itemKind === "object")) &&
                      store.objects.length > 0 && (
                        <select
                          className="doc-object-select"
                          value={p.refHash}
                          onChange={(e) => setProp(i, { ...p, refHash: e.target.value })}
                        >
                          <option value="">—</option>
                          {store.objects
                            .filter((o) => o.uuid !== selected.uuid)
                            .map((o) => (
                              <option key={o.hash} value={o.hash}>
                                {o.name}
                              </option>
                            ))}
                        </select>
                      )}
                    {p.refHash && (
                      <button
                        className="objects-ref-jump"
                        title={t("objects.refJump")}
                        onClick={() => {
                          const target = store.objects.find((x) => x.hash === p.refHash);
                          if (target) onSelectObject(target.uuid);
                        }}
                      >
                        →
                      </button>
                    )}
                  </td>
                  <td>
                    <label className="kv-check">
                      <input
                        type="checkbox"
                        checked={p.required}
                        onChange={(e) => setProp(i, { ...p, required: e.target.checked })}
                      />
                    </label>
                  </td>
                  <td>
                    <input
                      className="doc-name-input objects-prop-input"
                      value={p.description}
                      onChange={(e) => setProp(i, { ...p, description: e.target.value })}
                      spellCheck={false}
                    />
                  </td>
                  <td>
                    <button className="doc-op doc-op-del" onClick={() => removeProp(i)} title={t("common.delete")}>
                      ×
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {draft.properties.length === 0 && (
            <div className="doc-empty">
              {t("objects.noProps")}
              <button className="btn-link" onClick={addProp}>
                ＋ {t("objects.addProp")}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function fmt(ts: number): string {
  if (!ts) return "-";
  const d = new Date(ts * 1000);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
