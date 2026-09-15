import { Fragment, lazy, Suspense, useEffect, useState } from "react";
import { ApiFile, emptyNet } from "../types";
import { PacketEditorList } from "./PacketEditorList";
import { DescEditor } from "./Editor";
import { ExamplesTab } from "./ExamplesTab";
import { NetDoc } from "./NetDoc";
import { useT } from "../i18n";

// 代码生成页签懒加载（与 Editor 一致）
const CodeTab = lazy(() => import("./CodeTab").then((m) => ({ default: m.CodeTab })));

type Tab = "pack" | "unpack" | "desc" | "doc" | "code" | "examples";

/** 面包屑最多直接展示的层级数，超出时折叠中间层级为 … */
const BREADCRUMB_MAX = 5;

interface Props {
  api: ApiFile;
  baseUrl: string;
  /** 右侧接口面包屑：工作区名称 / …/ 分组名称 / 接口名称 */
  breadcrumb?: string[];
  currentVersion?: number;
  enableVersion: boolean;
  enableCodegen: boolean;
  codegenLang: string;
  sending: boolean;
  style?: React.CSSProperties;
  onChange: (a: ApiFile) => void;
  onSend: () => void;
  onSaveVersion: () => void;
  onCommit?: () => void;
  onTabChange?: (t: string) => void;
}

/**
 * TCP / UDP 接口编辑区：IP:Port 连接配置 + 封包 / 解包 / 描述 / 文档 / 代码生成 / 示例 六个页签。
 * 与 HTTP 编辑区（Editor）分离，避免 HTTP 专用页签（Query / Headers / Body / Mock 等）干扰。
 */
export function NetEditor({
  api,
  baseUrl,
  breadcrumb,
  currentVersion = 0,
  enableVersion,
  enableCodegen,
  codegenLang,
  sending,
  style,
  onChange,
  onSend,
  onSaveVersion,
  onCommit,
  onTabChange,
}: Props) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("pack");
  /** 示例数量：示例页签加载后回报，用于页签角标 */
  const [exampleCount, setExampleCount] = useState(0);

  const set = (p: Partial<ApiFile>) => onChange({ ...api, ...p });
  const net = api.net || emptyNet();

  const switchTab = (next: Tab) => {
    setTab(next);
    onTabChange?.(next);
  };

  // 切换接口时回到「封包」页签（并同步响应面板显隐）
  useEffect(() => {
    setTab("pack");
    onTabChange?.("pack");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api.uuid]);

  const crumbs = (() => {
    const list = breadcrumb || [];
    if (list.length <= BREADCRUMB_MAX) return list;
    return [list[0], "…", ...list.slice(list.length - (BREADCRUMB_MAX - 2))];
  })();

  return (
    <div className="editor" style={style}>
      {crumbs.length > 0 && (
        <div className="editor-crumbs" title={breadcrumb?.join(" / ")}>
          {crumbs.map((c, i) => (
            <Fragment key={i}>
              {i > 0 && <span className="crumb-sep">/</span>}
              <span
                className={`crumb${i === crumbs.length - 1 ? " current" : ""}${c === "…" ? " ellipsis" : ""}`}
              >
                {c}
              </span>
            </Fragment>
          ))}
        </div>
      )}
      <div className="editor-head">
        <div className="scheme-switch" title={t("editor.protocol")}>
          <button
            className={`scheme-btn${api.protocol === "tcp" ? " active" : ""}`}
            onClick={() => set({ protocol: "tcp" })}
          >
            TCP
          </button>
          <button
            className={`scheme-btn${api.protocol === "udp" ? " active" : ""}`}
            onClick={() => set({ protocol: "udp" })}
          >
            UDP
          </button>
        </div>
        <div className="net-addr">
          <span className="url-scheme">IP</span>
          <input
            className="url-input"
            value={net.host}
            placeholder="127.0.0.1"
            title={t("net.hostTip")}
            spellCheck={false}
            onChange={(e) => set({ net: { ...net, host: e.target.value } })}
          />
          <span className="net-colon">:</span>
          <input
            className="url-input net-port"
            type="number"
            min={1}
            max={65535}
            value={net.port}
            placeholder={api.protocol === "udp" ? "9101" : "9100"}
            title={t("net.portTip")}
            onChange={(e) => set({ net: { ...net, port: Math.max(0, Number(e.target.value) || 0) } })}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !sending) onSend();
            }}
          />
        </div>
        <label className="net-timeout" title={t("net.timeoutTip")}>
          {t("net.timeout")}
          <input
            className="url-input net-timeout-input"
            type="number"
            min={100}
            step={100}
            value={net.timeoutMs}
            onChange={(e) => set({ net: { ...net, timeoutMs: Math.max(100, Number(e.target.value) || 100) } })}
          />
        </label>
        <button className="send-btn" onClick={onSend} disabled={sending}>
          {sending ? t("tab.sending") : t("tab.send")}
        </button>
        {enableVersion && (
          <button
            className="save-btn"
            onClick={onSaveVersion}
            title={currentVersion > 0 ? t("tab.currentVersion", { v: currentVersion }) : t("tab.noVersion")}
          >
            💾 {t("tab.save")}
          </button>
        )}
      </div>

      <div className="tabs">
        <div className={`tab ${tab === "pack" ? "active" : ""}`} onClick={() => switchTab("pack")}>
          {t("net.packTab")}
          {(api.pack?.length ?? 0) > 0 && <span className="count">{api.pack?.length}</span>}
        </div>
        <div className={`tab ${tab === "unpack" ? "active" : ""}`} onClick={() => switchTab("unpack")}>
          {t("net.unpackTab")}
          {(api.unpack?.length ?? 0) > 0 && <span className="count">{api.unpack?.length}</span>}
        </div>
        <div className={`tab ${tab === "desc" ? "active" : ""}`} onClick={() => switchTab("desc")}>
          {t("editor.descTab")}
        </div>
        <div className={`tab ${tab === "doc" ? "active" : ""}`} onClick={() => switchTab("doc")}>
          {t("tab.doc")}
        </div>
        {enableCodegen && (
          <div className={`tab ${tab === "code" ? "active" : ""}`} onClick={() => switchTab("code")}>
            {t("editor.codeTab")}
          </div>
        )}
        <div className={`tab ${tab === "examples" ? "active" : ""}`} onClick={() => switchTab("examples")}>
          {t("tab.examples")}
          {exampleCount > 0 && <span className="count">{exampleCount}</span>}
        </div>
      </div>

      <div className="editor-body">
        {tab === "pack" && (
          <PacketEditorList fields={api.pack || []} onChange={(fields) => set({ pack: fields })} />
        )}
        {tab === "unpack" && (
          <PacketEditorList fields={api.unpack || []} onChange={(fields) => set({ unpack: fields })} />
        )}
        {tab === "desc" && (
          <DescEditor value={api.description} onChange={(v) => set({ description: v })} onCommit={onCommit} />
        )}
        {tab === "doc" && <NetDoc api={api} />}
        {tab === "code" && enableCodegen && (
          <Suspense fallback={<div className="tab-loading">{t("examples.loading")}</div>}>
            <CodeTab api={api} baseUrl={baseUrl} defaultLang={codegenLang} />
          </Suspense>
        )}
        {tab === "examples" && (
          <ExamplesTab
            uuid={api.uuid}
            api={api}
            onChange={onChange}
            onCountChange={setExampleCount}
          />
        )}
      </div>
    </div>
  );
}
