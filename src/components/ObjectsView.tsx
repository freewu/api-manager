import { useEffect, useMemo, useState } from "react";
import hljs from "highlight.js/lib/core";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import dart from "highlight.js/lib/languages/dart";
import delphi from "highlight.js/lib/languages/delphi";
import erlang from "highlight.js/lib/languages/erlang";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import julia from "highlight.js/lib/languages/julia";
import kotlin from "highlight.js/lib/languages/kotlin";
import objectivec from "highlight.js/lib/languages/objectivec";
import php from "highlight.js/lib/languages/php";
import python from "highlight.js/lib/languages/python";
import ruby from "highlight.js/lib/languages/ruby";
import rust from "highlight.js/lib/languages/rust";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import "highlight.js/styles/github-dark.css";
import { useT } from "../i18n";
import { renderMarkdown, saveObjectVersion } from "../commands";
import { LangSelect } from "./LangSelect";
import { OBJECT_LANGS, generateObjectCode } from "../utils/objectCodegen";
import MockPicker from "./MockPicker";
import ObjectRefPicker from "./ObjectRefPicker";

/** 对象名称（object_name）：字母开头，仅字母/数字/下划线 */
const OBJECT_NAME_RE = /^[A-Za-z][A-Za-z0-9_]*$/;
/** Java 包名（package_name）：小写字母开头，点分隔，每段字母/数字/下划线 */
const PACKAGE_NAME_RE = /^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$/;
import {
  ObjectDef,
  ObjectImportResult,
  ObjectProp,
  ObjectStore,
  PROP_KINDS,
} from "../types";

for (const [name, lang] of [
  ["c", c],
  ["cpp", cpp],
  ["csharp", csharp],
  ["dart", dart],
  ["delphi", delphi],
  ["erlang", erlang],
  ["go", go],
  ["java", java],
  ["julia", julia],
  ["kotlin", kotlin],
  ["objectivec", objectivec],
  ["php", php],
  ["python", python],
  ["ruby", ruby],
  ["rust", rust],
  ["swift", swift],
  ["typescript", typescript],
] as const) {
  hljs.registerLanguage(name, lang);
}

interface Props {
  store: ObjectStore;
  onSave: (store: ObjectStore) => Promise<ObjectStore>;
  onImport: (name: string, group: string, json: string) => Promise<ObjectImportResult>;
  onImportDdl: (group: string, ddl: string) => Promise<ObjectImportResult>;
  onJumpApi: (path: string) => void;
  onToast: (msg: string) => void;
  /** 空状态：请求打开左侧新建对象弹窗 */
  onRequestNew: () => void;
  /** 空状态：请求打开左侧导入（粘贴 JSON）弹窗 */
  onRequestImport: () => void;
  /** 当前选中对象 uuid（null = 未选中） */
  selectedUuid: string | null;
  onSelectObject: (uuid: string | null) => void;
  /** 代码生成默认语言（来自设置页 codegenLang） */
  defaultCodeLang: string;
}

const ITEM_KINDS = ["string", "number", "boolean", "datetime", "date", "time", "object", "any"];

function emptyProp(): ObjectProp {
  return { key: "", kind: "string", itemKind: "", refHash: "", description: "", mock: "" };
}

/** 对象管理：右侧对象配置（类似接口文档管理的响应编辑，kv 表格风格） */
export default function ObjectsView({
  store,
  onSave,
  onImport: _onImport,
  onImportDdl: _onImportDdl,
  onJumpApi: _onJumpApi,
  onToast,
  onRequestNew,
  onRequestImport,
  selectedUuid,
  onSelectObject,
  defaultCodeLang,
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

  // 右侧 tab：属性 / 对象描述 / 代码生成
  const [tab, setTab] = useState<"props" | "desc" | "code">("props");
  // 默认语言取设置页 codegenLang（不在对象语言列表时回退第一个）
  const [codeLang, setCodeLang] = useState(() =>
    OBJECT_LANGS.some((l) => l.value === defaultCodeLang) ? defaultCodeLang : OBJECT_LANGS[0].value
  );
  // 设置页修改默认语言后同步（用户手动切换不被覆盖）
  useEffect(() => {
    if (OBJECT_LANGS.some((l) => l.value === defaultCodeLang) && defaultCodeLang !== codeLang) {
      setCodeLang(defaultCodeLang);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [defaultCodeLang]);
  /** 正在选择 mock 占位符的属性下标（null = 未打开） */
  const [mockPickIndex, setMockPickIndex] = useState<number | null>(null);
  /** 正在选择引用对象的属性下标（null = 未打开） */
  const [refPickIndex, setRefPickIndex] = useState<number | null>(null);
  /** Java 展示风格 tab：lombok / native（切换显示，均同时生成） */
  const [javaStyle, setJavaStyle] = useState<"lombok" | "native">("lombok");
  /** Java 双风格代码（lombok / native 同时生成） */
  const codeJava = useMemo(() => {
    if (!draft || codeLang !== "java") return null;
    // 引用对象解析：以最新草稿替换同 uuid 的 store 对象
    const all = store.objects.map((o) => (o.uuid === draft.uuid ? draft : o));
    return {
      lombok: generateObjectCode("java", draft, all, { javaStyle: "lombok" }),
      native: generateObjectCode("java", draft, all, { javaStyle: "native" }),
    };
  }, [draft, store.objects, codeLang]);
  /** 其他语言单段代码 */
  const codeOther = useMemo(() => {
    if (!draft || codeLang === "java") return "";
    const all = store.objects.map((o) => (o.uuid === draft.uuid ? draft : o));
    return generateObjectCode(codeLang, draft, all);
  }, [draft, store.objects, codeLang]);
  /** 复制用全文（java 时拼接两种风格，注释分隔） */
  const codeAll = useMemo(() => {
    if (codeLang === "java") {
      if (!codeJava) return "";
      const parts: string[] = [];
      if (codeJava.lombok) parts.push(`// ===== Lombok =====\n${codeJava.lombok}`);
      if (codeJava.native) parts.push(`// ===== 原生（getter/setter） =====\n${codeJava.native}`);
      return parts.join("\n\n");
    }
    return codeOther;
  }, [codeLang, codeJava, codeOther]);
  const highlight = (text: string, lang: string): string => {
    try {
      return hljs.highlight(text, { language: lang }).value;
    } catch {
      return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }
  };
  const [copied, setCopied] = useState(false);
  const copyCode = async () => {
    try {
      await navigator.clipboard.writeText(codeAll);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* 剪贴板不可用时忽略 */
    }
  };

  if (!selected || !draft) {
    // 完全没有对象：空状态引导（新增 / 导入）
    if (store.objects.length === 0) {
      return (
        <div className="objects-blank objects-blank-empty">
          <div className="objects-empty-icon">🗂️</div>
          <div className="objects-empty-title">{t("objects.emptyTitle")}</div>
          <div className="objects-empty-sub">{t("objects.emptySub")}</div>
          <div className="objects-empty-actions">
            <button className="btn primary" onClick={onRequestNew}>
              ＋ {t("objects.newObject")}
            </button>
            <button className="btn" onClick={onRequestImport}>
              📥 {t("objects.importObject")}
            </button>
          </div>
        </div>
      );
    }
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
    // 校验：对象名称 / 包名格式
    if (draft.object_name && !OBJECT_NAME_RE.test(draft.object_name)) {
      onToast(t("objects.objectNameInvalid"));
      return;
    }
    if (draft.package_name && !PACKAGE_NAME_RE.test(draft.package_name)) {
      onToast(t("objects.packageNameInvalid"));
      return;
    }
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


  return (
    <div className="history-view-content">
      <div className="history-detail objects-detail">
        {/* 头部：对象包名 / 对象名称 / 保存 */}
        <div className="objects-detail-head">
          <div className="objects-head-field">
            <span className="objects-head-label">{t("objects.packageName")}</span>
            <input
              className="objects-name-input"
              value={draft.package_name || ""}
              onChange={(e) => patch((d) => void (d.package_name = e.target.value.trim() || undefined))}
              placeholder={t("objects.packageNamePh")}
              spellCheck={false}
            />
            {!!draft.package_name && !PACKAGE_NAME_RE.test(draft.package_name) && (
              <span className="objects-name-error">{t("objects.packageNameInvalid")}</span>
            )}
          </div>
          <div className="objects-head-field">
            <span className="objects-head-label">{t("objects.objectName")}</span>
            <input
              className="objects-name-input"
              value={draft.object_name || ""}
              onChange={(e) => patch((d) => void (d.object_name = e.target.value.trim() || undefined))}
              placeholder={t("objects.objectNamePh")}
              spellCheck={false}
            />
            {!!draft.object_name && !OBJECT_NAME_RE.test(draft.object_name) && (
              <span className="objects-name-error">{t("objects.objectNameInvalid")}</span>
            )}
          </div>
          <button className="btn primary objects-save-btn" disabled={!dirty} onClick={() => void save()}>
            {t("objects.save")}
          </button>
        </div>

        {/* Tab 栏：属性 / 对象描述 / 代码生成 */}
        <div className="objects-tabs">
          <button
            className={`objects-tab${tab === "props" ? " active" : ""}`}
            onClick={() => setTab("props")}
          >
            {t("objects.tabProps")}
          </button>
          <button
            className={`objects-tab${tab === "desc" ? " active" : ""}`}
            onClick={() => setTab("desc")}
          >
            {t("objects.tabDesc")}
          </button>
          <button
            className={`objects-tab${tab === "code" ? " active" : ""}`}
            onClick={() => setTab("code")}
          >
            {t("objects.tabCode")}
          </button>
        </div>

        {tab === "desc" && (
          <ObjectsDescEditor
            value={draft.description}
            onChange={(v) => patch((d) => void (d.description = v))}
          />
        )}

        {tab === "props" && (
          <>
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
                <th style={{ width: "20%" }}>{t("objects.propDesc")}</th>
                <th style={{ width: "15%" }}>{t("objects.propMock")}</th>
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
                        <button
                          className="objects-ref-pick"
                          title={t("objects.refPick")}
                          onClick={() => setRefPickIndex(i)}
                        >
                          {(() => {
                            const ref = p.refHash ? store.objects.find((x) => x.hash === p.refHash) : undefined;
                            return ref ? (ref.displayName || ref.name) : t("objects.refPick");
                          })()}
                        </button>
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
                    <input
                      className="doc-name-input objects-prop-input"
                      value={p.description}
                      onChange={(e) => setProp(i, { ...p, description: e.target.value })}
                      spellCheck={false}
                    />
                  </td>
                  <td>
                    <div className="objects-mock-cell">
                      <input
                        className="doc-name-input objects-prop-input objects-prop-mock"
                        value={p.mock}
                        onChange={(e) => setProp(i, { ...p, mock: e.target.value })}
                        placeholder={t("objects.propMockPh")}
                        spellCheck={false}
                      />
                      <button
                        className="objects-mock-pick"
                        title={t("objects.mockPick")}
                        onClick={() => setMockPickIndex(i)}
                      >
                        ⚡
                      </button>
                    </div>
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
          </>
        )}

        {tab === "code" && (
          <div className="objects-codegen">
            <div className="objects-codegen-head">
              <LangSelect
                value={codeLang}
                options={OBJECT_LANGS.map((l) => ({ value: l.value, label: l.label }))}
                title={t("codegen.switchLang")}
                onChange={setCodeLang}
              />
              <button className="btn small" onClick={() => void copyCode()}>
                {copied ? t("resp.copied") : "📋 " + t("common.copy")}
              </button>
              <span className="objects-codegen-tip">{t("objects.codegenTip")}</span>
            </div>
            {codeLang === "java" && (
              <div className="objects-java-style">
                <label className={`objects-style-pill${javaStyle === "lombok" ? " active" : ""}`}>
                  <input
                    type="radio"
                    name="javaStyle"
                    checked={javaStyle === "lombok"}
                    onChange={() => setJavaStyle("lombok")}
                  />
                  {t("objects.javaStyleLombok")}
                </label>
                <label className={`objects-style-pill${javaStyle === "native" ? " active" : ""}`}>
                  <input
                    type="radio"
                    name="javaStyle"
                    checked={javaStyle === "native"}
                    onChange={() => setJavaStyle("native")}
                  />
                  {t("objects.javaStyleNative")}
                </label>
              </div>
            )}
            {codeLang === "java" && codeJava ? (
              codeJava[javaStyle] ? (
                <pre className="objects-codegen-pre">
                  <code dangerouslySetInnerHTML={{ __html: highlight(codeJava[javaStyle], "java") }} />
                </pre>
              ) : (
                <div className="objects-codegen-empty">{t("objects.codegenNoName")}</div>
              )
            ) : codeOther ? (
              <pre className="objects-codegen-pre">
                <code dangerouslySetInnerHTML={{ __html: highlight(codeOther, codeLang) }} />
              </pre>
            ) : (
              <div className="objects-codegen-empty">{t("objects.codegenNoName")}</div>
            )}
          </div>
        )}
      </div>
      {mockPickIndex !== null && draft && draft.properties[mockPickIndex] && (
        <MockPicker
          onPick={(v) => {
            setProp(mockPickIndex, { ...draft.properties[mockPickIndex], mock: v });
            setMockPickIndex(null);
          }}
          onClose={() => setMockPickIndex(null)}
        />
      )}
      {refPickIndex !== null && draft && draft.properties[refPickIndex] && (
        <ObjectRefPicker
          store={store}
          excludeUuid={selected.uuid}
          currentHash={draft.properties[refPickIndex].refHash}
          onPick={(hash) => {
            setProp(refPickIndex, { ...draft.properties[refPickIndex], refHash: hash });
            setRefPickIndex(null);
          }}
          onClose={() => setRefPickIndex(null)}
        />
      )}
    </div>
  );
}

/** 对象描述：Markdown 编辑 / 预览切换（预览由后端 md_to_html 渲染） */
function ObjectsDescEditor({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const t = useT();
  const [mode, setMode] = useState<"edit" | "preview">("edit");
  const [html, setHtml] = useState("");
  const [busy, setBusy] = useState(false);

  const toPreview = async () => {
    setBusy(true);
    try {
      setHtml(await renderMarkdown(value || ""));
      setMode("preview");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="objects-desc-root">
      <div className="objects-desc-toolbar">
        <button
          className={`objects-desc-mode${mode === "edit" ? " active" : ""}`}
          onClick={() => setMode("edit")}
        >
          ✏️ {t("editor.descEdit")}
        </button>
        <button
          className={`objects-desc-mode${mode === "preview" ? " active" : ""}`}
          disabled={busy}
          onClick={() => void toPreview()}
        >
          👁 {t("editor.descPreview")}
        </button>
      </div>
      {mode === "edit" ? (
        <textarea
          className="objects-desc-textarea"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          rows={14}
          placeholder={t("objects.description")}
          spellCheck={false}
        />
      ) : (
        <div className="objects-desc-preview md-preview" dangerouslySetInnerHTML={{ __html: html }} />
      )}
    </div>
  );
}
