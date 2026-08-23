import { useEffect, useState } from "react";
import { listObjectVersions, readObjectVersion } from "../commands";
import { ObjectDef, ObjectVersionInfo } from "../types";
import { Modal } from "./Modal";
import { useT } from "../i18n";

interface Props {
  /** 当前对象（用于与历史版本比对） */
  current: ObjectDef;
  onClose: () => void;
}

interface DiffRow {
  type: "same" | "add" | "del";
  a?: string; // 当前版本（左）
  b?: string; // 历史版本（右）
  an?: number;
  bn?: number;
}

// 基于 LCS 的行级 diff
function diffLines(a: string[], b: string[]): DiffRow[] {
  const n = a.length;
  const m = b.length;
  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const out: DiffRow[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      out.push({ type: "same", a: a[i], b: b[i] });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      out.push({ type: "del", a: a[i] });
      i++;
    } else {
      out.push({ type: "add", b: b[j] });
      j++;
    }
  }
  while (i < n) out.push({ type: "del", a: a[i++] });
  while (j < m) out.push({ type: "add", b: b[j++] });

  let an = 0;
  let bn = 0;
  return out.map((r) => {
    if (r.type === "add") {
      bn++;
      return { ...r, bn };
    }
    if (r.type === "del") {
      an++;
      return { ...r, an };
    }
    an++;
    bn++;
    return { ...r, an, bn };
  });
}

function fmtTime(sec: number): string {
  if (!sec) return "";
  const d = new Date(sec * 1000);
  const p = (x: number) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** 对象版本查看：版本列表 + 当前对象与选中版本的行级比对 */
export function ObjectVersionModal({ current, onClose }: Props) {
  const t = useT();
  const [versions, setVersions] = useState<ObjectVersionInfo[]>([]);
  const [selIdx, setSelIdx] = useState(0);
  const [diff, setDiff] = useState<DiffRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState("");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await listObjectVersions(current.uuid);
        if (cancelled) return;
        setVersions(list);
        setSelIdx(list.length - 1);
      } catch (e) {
        if (!cancelled) setErr(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [current.uuid]);

  const sel = versions[selIdx];

  useEffect(() => {
    if (!sel) {
      setDiff(null);
      return;
    }
    let cancelled = false;
    (async () => {
      setLoading(true);
      setErr("");
      setDiff(null);
      try {
        const vObj = await readObjectVersion(current.uuid, sel.version);
        const cur = JSON.stringify(current, null, 2).split("\n");
        const old = JSON.stringify(vObj, null, 2).split("\n");
        if (!cancelled) setDiff(diffLines(cur, old));
      } catch (e) {
        if (!cancelled) setErr(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selIdx, sel?.version]);

  return (
    <Modal
      title={`📑 ${t("version.title")} - ${current.name}`}
      onClose={onClose}
      className="modal-version"
      footer={
        <button className="btn" onClick={onClose}>
          {t("common.close")}
        </button>
      }
    >
      <div className="version-body">
        <div className="version-list">
          <div className="version-list-title">{t("version.history", { count: versions.length })}</div>
          {versions.length === 0 && (
            <div className="version-empty">
              {t("version.empty")}
              <br />
              {t("version.emptyHint")}
            </div>
          )}
          {versions.map((v, i) => (
            <div
              key={v.version}
              className={`version-item ${i === selIdx ? "active" : ""}`}
              onClick={() => setSelIdx(i)}
            >
              <div className="version-item-title">v{v.version}</div>
              <div className="version-item-meta">{fmtTime(v.savedAt)}</div>
              <div className="version-item-meta">
                {t("objects.props")}: {v.propCount}
              </div>
              {v.description && <div className="version-item-meta">{v.description}</div>}
            </div>
          ))}
        </div>
        <div className="diff-pane">
          <div className="diff-head">
            <span>
              <span className="diff-legend-add">＋ {t("version.current")}</span>
              <span className="diff-legend-del">− v{sel?.version ?? "?"}</span>
            </span>
            <span style={{ fontSize: 11, color: "var(--text-faint)" }}>{t("version.compare")}</span>
          </div>
          <div className="diff-body">
            {loading && <div className="diff-hint">{t("version.loading")}</div>}
            {err && <div className="diff-error">{err}</div>}
            {diff &&
              diff.map((r, i) => (
                <div key={i} className={`diff-row ${r.type}`}>
                  <span className="diff-num">{r.an ?? ""}</span>
                  <span className="diff-num">{r.bn ?? ""}</span>
                  <span className="diff-mark">{r.type === "del" ? "+" : r.type === "add" ? "-" : " "}</span>
                  <span className="diff-text">{r.type === "add" ? r.b : r.type === "del" ? r.a : r.a}</span>
                </div>
              ))}
          </div>
        </div>
      </div>
    </Modal>
  );
}
