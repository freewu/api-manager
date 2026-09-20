import { useMemo, useState } from "react";
import { NetResult, PacketField } from "../../../types";
import { bytesToText, hexToBytes, parsePacket } from "../../../utils/packet";
import { PacketViewTable } from "../common/PacketViewTable";
import { CopyBtn } from "../common/CopyBtn";
import { useT } from "../../../i18n";

interface Props {
  result: NetResult | null;
  sending: boolean;
  /** 解包字段定义（响应报文的解析规则） */
  fields: PacketField[];
  onSaveExample?: (name: string) => void;
}

/**
 * TCP / UDP 响应面板：展示请求封包 bytes、响应原始字节与文本，以及按「解包」字段定义解析出的字段表。
 */
export function NetResponse({ result, sending, fields, onSaveExample }: Props) {
  const t = useT();
  const [tab, setTab] = useState<"unpack" | "raw">("unpack");
  const [saveOpen, setSaveOpen] = useState(false);
  const [saveName, setSaveName] = useState("");

  const rows = useMemo(
    () => (result && result.hex ? parsePacket(fields, hexToBytes(result.hex)) : []),
    [result, fields]
  );

  if (sending && !result) {
    return (
      <div className="response">
        <div className="response-body">
          <div className="response-empty">
            <span className="big">⏳</span>
            <span>{t("resp.sending")}</span>
          </div>
        </div>
      </div>
    );
  }

  if (!result) {
    return (
      <div className="response">
        <div className="response-head" style={{ color: "var(--text-faint)" }}>
          {t("resp.response")}
        </div>
        <div className="response-body">
          <div className="response-empty">
            <span className="big">📡</span>
            <span>{t("net.hint")}</span>
          </div>
        </div>
      </div>
    );
  }

  const text = result.text?.trim() ? result.text : bytesToText(hexToBytes(result.hex));

  return (
    <div className="response">
      <div className="response-head">
        <span className={`status-badge ${result.ok ? "status-2xx" : "status-5xx"}`}>
          {result.ok ? t("net.ok") : t("resp.failed")}
        </span>
        <span className="resp-meta">
          <span>
            <span className="label">{t("resp.time")} </span>
            <b>{result.timeMs} ms</b>
          </span>
          <span>
            <span className="label">{t("net.size")} </span>
            <b>
              {t("net.send")} {result.sentSize} / {t("net.recv")} {result.size} {t("net.byte")}
            </b>
          </span>
          {result.from && (
            <span>
              <span className="label">{t("net.from")} </span>
              <b>{result.from}</b>
            </span>
          )}
        </span>
        {onSaveExample && (
          <div className="resp-save-example">
            {saveOpen ? (
              <>
                <input
                  className="resp-save-input"
                  autoFocus
                  placeholder={t("resp.exampleName")}
                  value={saveName}
                  onChange={(e) => setSaveName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && saveName.trim()) {
                      onSaveExample(saveName.trim());
                      setSaveOpen(false);
                      setSaveName("");
                    }
                    if (e.key === "Escape") {
                      setSaveOpen(false);
                      setSaveName("");
                    }
                  }}
                />
                <button
                  type="button"
                  className="btn small primary"
                  disabled={!saveName.trim()}
                  onClick={() => {
                    if (saveName.trim()) {
                      onSaveExample(saveName.trim());
                      setSaveOpen(false);
                      setSaveName("");
                    }
                  }}
                >
                  {t("common.save")}
                </button>
                <button type="button" className="btn small" onClick={() => setSaveOpen(false)}>
                  {t("common.cancel")}
                </button>
              </>
            ) : (
              <button type="button" className="btn small" onClick={() => setSaveOpen(true)}>
                💾 {t("resp.saveExample")}
              </button>
            )}
          </div>
        )}
      </div>
      <div className="response-body">
        {result.error && <div className="error-banner">{result.error}</div>}
        <div className="resp-tabs">
          <div className={`resp-tab ${tab === "unpack" ? "active" : ""}`} onClick={() => setTab("unpack")}>
            {t("net.unpackTab")}
          </div>
          <div className={`resp-tab ${tab === "raw" ? "active" : ""}`} onClick={() => setTab("raw")}>
            {t("net.rawTab")}
          </div>
        </div>

        {tab === "unpack" &&
          (fields.length === 0 ? (
            <div className="response-empty">
              <span>{t("net.unpackEmpty")}</span>
            </div>
          ) : result.size === 0 ? (
            <div className="response-empty">
              <span>{t("net.noResponse")}</span>
            </div>
          ) : (
            <PacketViewTable fields={fields} rows={rows} />
          ))}

        {tab === "raw" && (
          <div className="net-raw">
            <div className="section-title">
              {t("net.sentBytes")}
              <CopyBtn className="copy-inline" text={result.sentHex} title={t("net.copyPreviewTip")} />
            </div>
            <code className="packet-hex">{result.sentHex || t("net.previewEmpty")}</code>
            <div className="section-title">
              {t("net.recvBytes")}
              <CopyBtn className="copy-inline" text={result.hex} title={t("net.copyPreviewTip")} />
            </div>
            <code className="packet-hex">{result.hex || t("net.previewEmpty")}</code>
            {text && (
              <>
                <div className="section-title">{t("net.textPreview")}</div>
                <div className="json-view raw-view">{text}</div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
