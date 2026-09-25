import { Fragment, lazy, Suspense, useEffect, useState } from "react";
import { ApiFile, BodyData, MQ_KINDS, MqKind, emptyMq } from "../../../types";
import { DescEditor } from "./DescEditor";
import { ExamplesTab } from "./ExamplesTab";
import { MqDoc } from "./MqDoc";
import { MqKindSelect } from "./MqKindSelect";
import { listExamples, saveExample } from "../../../commands";
import { useT } from "../../../i18n";

// 代码生成页签懒加载（与 Editor / NetEditor 一致）
const CodeTab = lazy(() => import("../common/CodeTab").then((m) => ({ default: m.CodeTab })));

type Tab = "produce" | "consume" | "doc" | "code" | "examples";

/** 面包屑最多直接展示的层级数，超出时折叠中间层级为 … */
const BREADCRUMB_MAX = 5;

/** 生产消息的格式（仅影响示例与文档展示，MQ 发送的是字符串） */
const MSG_MODES: BodyData["mode"][] = ["raw", "json", "xml"];

interface Props {
  api: ApiFile;
  baseUrl: string;
  /** 右侧接口面包屑：工作区名称 / …/ 分组名称 / 接口名称 */
  breadcrumb?: string[];
  currentVersion?: number;
  enableVersion: boolean;
  enableCodegen: boolean;
  codegenLang: string;
  style?: React.CSSProperties;
  onChange: (a: ApiFile) => void;
  onSaveVersion: () => void;
  onCommit?: () => void;
  onTabChange?: (t: string) => void;
}

/**
 * MQ（消息队列）接口编辑区：MQ 类型 / IP:Port / Topic 连接配置 + 生产 / 消费 / 文档 / 代码生成 / 示例 五个页签。
 * MQ 接口不直接连接 Broker（无发送按钮），只用于维护连接配置与生成生产 / 消费代码。
 */
export function MqEditor({
  api,
  baseUrl,
  breadcrumb,
  currentVersion = 0,
  enableVersion,
  enableCodegen,
  codegenLang,
  style,
  onChange,
  onSaveVersion,
  onCommit,
  onTabChange,
}: Props) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("produce");
  /** 示例数量：示例页签加载后回报，用于页签角标 */
  const [exampleCount, setExampleCount] = useState(0);
  /** 保存示例的提示与错误（保存按钮在「生产」页签） */
  const [saveMsg, setSaveMsg] = useState("");
  const [saveErr, setSaveErr] = useState("");

  const set = (p: Partial<ApiFile>) => onChange({ ...api, ...p });
  const mq = api.mq || emptyMq();
  const body = api.body || { mode: "raw" as BodyData["mode"], raw: "", form: [], binaryPath: "" };

  const switchTab = (next: Tab) => {
    setTab(next);
    onTabChange?.(next);
  };

  // 切换接口时回到「生产」页签（并同步响应面板显隐）
  useEffect(() => {
    setTab("produce");
    onTabChange?.("produce");
    setSaveMsg("");
    setSaveErr("");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api.uuid]);

  /** 切换 MQ 类型：同时把端口改为该类型的默认端口（仅当原端口为空或为上一类型的默认端口） */
  const changeKind = (kind: MqKind) => {
    const prevDefault = MQ_KINDS.find((k) => k.value === mq.type)?.port;
    const nextDefault = MQ_KINDS.find((k) => k.value === kind)?.port ?? 9092;
    const keepPort = mq.port > 0 && mq.port !== prevDefault;
    set({ mq: { ...mq, type: kind, port: keepPort ? mq.port : nextDefault } });
  };

  const crumbs = (() => {
    const list = breadcrumb || [];
    if (list.length <= BREADCRUMB_MAX) return list;
    return [list[0], "…", ...list.slice(list.length - (BREADCRUMB_MAX - 2))];
  })();

  /** 保存当前 MQ 配置与消息内容为示例（MQ 无请求 / 响应，示例只记录配置与消息） */
  const saveAsExample = async () => {
    setSaveErr("");
    setSaveMsg("");
    const name = `${mq.type || "mq"} ${mq.topic || `${mq.host}:${mq.port}`}`.trim();
    if (!api.uuid) {
      setSaveErr(t("mq.exampleFailed", { msg: "uuid" }));
      return;
    }
    try {
      const now = Math.floor(Date.now() / 1000);
      await saveExample(api.uuid, name, {
        name,
        time: now,
        method: "MQ",
        url: `${mq.type}://${mq.host}:${mq.port}/${mq.topic}`,
        reqHeaders: [],
        reqPath: [],
        reqQuery: [],
        reqBody: body.raw || "",
        status: 0,
        statusText: "",
        respHeaders: [],
        respBody: "",
        timeMs: 0,
        size: 0,
        protocol: "mq",
        mq: { ...mq },
      });
      const list = await listExamples(api.uuid);
      setExampleCount(list.length);
      setSaveMsg(t("mq.exampleSaved"));
      window.setTimeout(() => setSaveMsg((p) => (p ? "" : p)), 3000);
    } catch (e) {
      setSaveErr(t("mq.exampleFailed", { msg: String(e) }));
    }
  };

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
      <div className="editor-head mq-head">
        {/* MQ 类型：原生 select 无法展示图标，改用 dl/dd 自绘下拉（触发区与选项都带品牌图标） */}
        <MqKindSelect
          value={mq.type}
          options={MQ_KINDS}
          onChange={changeKind}
          title={t("mq.kindTip")}
          ariaLabel={t("mq.kind")}
        />
        <div className="net-addr">
          <span className="url-scheme">IP</span>
          <input
            className="url-input"
            value={mq.host}
            placeholder="127.0.0.1"
            title={t("net.hostTip")}
            spellCheck={false}
            onChange={(e) => set({ mq: { ...mq, host: e.target.value } })}
          />
          <span className="net-colon">:</span>
          <input
            className="url-input net-port"
            type="number"
            min={1}
            max={65535}
            value={mq.port}
            placeholder="9092"
            title={t("net.portTip")}
            onChange={(e) => set({ mq: { ...mq, port: Math.max(0, Number(e.target.value) || 0) } })}
          />
        </div>
        <div className="url-input-wrap mq-topic">
          <span className="url-scheme">{t("mq.topic")}</span>
          <input
            className="url-input"
            value={mq.topic}
            placeholder={t("mq.topicPlaceholder")}
            title={t("mq.topicTip")}
            spellCheck={false}
            onChange={(e) => set({ mq: { ...mq, topic: e.target.value } })}
          />
        </div>
        <label className="net-timeout" title={t("net.timeoutTip")}>
          {t("net.timeout")}
          <input
            className="url-input net-timeout-input"
            type="number"
            min={100}
            step={100}
            value={mq.timeoutMs}
            onChange={(e) => set({ mq: { ...mq, timeoutMs: Math.max(100, Number(e.target.value) || 100) } })}
          />
        </label>
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
        <div className={`tab ${tab === "produce" ? "active" : ""}`} onClick={() => switchTab("produce")}>
          {t("mq.produceTab")}
        </div>
        <div className={`tab ${tab === "consume" ? "active" : ""}`} onClick={() => switchTab("consume")}>
          {t("mq.consumeTab")}
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
        {tab === "produce" && (
          <div className="mq-pane">
            <div className="mq-pane-head">
              <span className="section-title">{t("mq.msgBody")}</span>
              <label className="mq-format">
                {t("mq.msgFormat")}
                <select
                  className="mq-format-select"
                  value={body.mode === "json" || body.mode === "xml" ? body.mode : "raw"}
                  onChange={(e) =>
                    set({ body: { ...body, mode: e.target.value as BodyData["mode"] } })
                  }
                >
                  {MSG_MODES.map((m) => (
                    <option key={m} value={m}>
                      {m.toUpperCase()}
                    </option>
                  ))}
                </select>
              </label>
              <button className="btn small" onClick={() => void saveAsExample()} title={t("mq.saveExampleTip")}>
                💾 {t("mq.saveExample")}
              </button>
              {saveMsg && <span className="mq-saved">✔ {saveMsg}</span>}
              {saveErr && <span className="body-format-error">{saveErr}</span>}
            </div>
            <div className="body-raw-wrap">
              <div className="body-raw-toolbar">
                <span className="help">{mqKindLabelOf(mq.type, body.mode)}</span>
              </div>
              <textarea
                className="code-area"
                value={body.raw}
                placeholder={t("mq.msgPlaceholder")}
                spellCheck={false}
                onChange={(e) => set({ body: { ...body, raw: e.target.value } })}
              />
            </div>
            <div className="mq-hint">{t("mq.hint")}</div>
          </div>
        )}

        {tab === "consume" && (
          <div className="mq-pane">
            <div className="mq-form">
              <label className="mq-field" title={t("mq.consumeGroupTip")}>
                <span className="label">{t("mq.consumeGroup")}</span>
                <input
                  className="url-input"
                  value={mq.group}
                  placeholder={t("mq.consumeGroupPlaceholder")}
                  spellCheck={false}
                  onChange={(e) => set({ mq: { ...mq, group: e.target.value } })}
                />
              </label>
              <label className="mq-field" title={t("mq.offsetTip")}>
                <span className="label">{t("mq.offset")}</span>
                <select
                  className="mq-kind-select"
                  value={mq.offset}
                  onChange={(e) =>
                    set({ mq: { ...mq, offset: e.target.value === "earliest" ? "earliest" : "latest" } })
                  }
                >
                  <option value="latest">{t("mq.offsetLatest")}</option>
                  <option value="earliest">{t("mq.offsetEarliest")}</option>
                </select>
              </label>
              <label className="mq-field" title={t("mq.maxMessagesTip")}>
                <span className="label">{t("mq.maxMessages")}</span>
                <input
                  className="url-input mq-num"
                  type="number"
                  min={1}
                  value={mq.maxMessages}
                  onChange={(e) =>
                    set({ mq: { ...mq, maxMessages: Math.max(1, Number(e.target.value) || 1) } })
                  }
                />
              </label>
              <label className="mq-field" title={t("net.timeoutTip")}>
                <span className="label">{t("net.timeout")}</span>
                <input
                  className="url-input mq-num"
                  type="number"
                  min={100}
                  step={100}
                  value={mq.timeoutMs}
                  onChange={(e) =>
                    set({ mq: { ...mq, timeoutMs: Math.max(100, Number(e.target.value) || 100) } })
                  }
                />
              </label>
            </div>
            <div className="mq-hint">{t("mq.hint")}</div>
          </div>
        )}

        {tab === "doc" && (
          <div className="mq-pane mq-doc-pane">
            <div className="section-title">{t("editor.descTab")}</div>
            <DescEditor value={api.description} onChange={(v) => set({ description: v })} onCommit={onCommit} />
            <MqDoc api={api} />
          </div>
        )}

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

/** 消息格式展示名（如 Kafka · JSON） */
function mqKindLabelOf(kind: MqKind, mode: string): string {
  const label = MQ_KINDS.find((k) => k.value === kind)?.label ?? "MQ";
  return `${label} · ${mode.toUpperCase()}`;
}
