import { useCallback, useState } from "react";
import {
  HistoryDay,
  HistoryDetail,
  HistorySummary,
  historyClear,
  historyDays,
  historyDetail,
  historyRecords,
} from "../commands";

const PAGE = 100;

export interface HistoryDiffPair {
  a: HistoryDetail;
  b: HistoryDetail;
}

/**
 * 请求历史列表状态：分页懒加载（每页 100 条）+ 选中记录详情 + Diff 比对
 * 列表加载是惰性的：第一次进入历史视图时才拉取
 */
export function useHistory() {
  const [records, setRecords] = useState<HistorySummary[]>([]);
  const [days, setDays] = useState<HistoryDay[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<HistoryDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  // Diff 比对：比对模式开关 + 已勾选的记录 id（最多 2 条，且必须属于同一接口 uuid）
  const [diffMode, setDiffMode] = useState(false);
  const [diffIds, setDiffIds] = useState<string[]>([]);
  const [diffPair, setDiffPair] = useState<HistoryDiffPair | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [diffError, setDiffError] = useState("");

  const loadPage = useCallback(async (start: number) => {
    setLoading(true);
    try {
      const list = await historyRecords(start, PAGE);
      setRecords((prev) => (start === 0 ? list : [...prev, ...list]));
      setOffset(start + list.length);
      setHasMore(list.length === PAGE);
    } catch (e) {
      console.error(e);
      setHasMore(false);
    } finally {
      setLoading(false);
    }
  }, []);

  const reload = useCallback(() => {
    void loadPage(0);
    historyDays().then(setDays).catch(() => {});
    setLoaded(true);
  }, [loadPage]);

  const select = useCallback(async (id: string) => {
    setSelectedId(id);
    setDetailLoading(true);
    try {
      setDetail(await historyDetail(id));
    } catch (e) {
      console.error(e);
      setDetail(null);
    } finally {
      setDetailLoading(false);
    }
  }, []);

  const clearAll = useCallback(async () => {
    try {
      await historyClear();
      setRecords([]);
      setDays([]);
      setHasMore(false);
      setOffset(0);
      setDetail(null);
      setSelectedId(null);
      setDiffMode(false);
      setDiffIds([]);
      setDiffPair(null);
    } catch (e) {
      console.error(e);
    }
  }, []);

  /** 进入/退出比对模式 */
  const toggleDiffMode = useCallback((on: boolean) => {
    setDiffMode(on);
    setDiffIds([]);
    setDiffPair(null);
    setDiffError("");
  }, []);

  /** 勾选/取消勾选一条记录。约束：最多 2 条且必须属于同一接口 uuid */
  const toggleDiffSelect = useCallback((r: HistorySummary) => {
    setDiffPair(null);
    setDiffError("");
    setDiffIds((prev) => {
      const i = prev.indexOf(r.id);
      if (i >= 0) return prev.filter((x) => x !== r.id);
      if (prev.length >= 2) return prev;
      if (prev.length === 1) {
        const first = records.find((x) => x.id === prev[0]);
        const sameApi = first && first.apiUuid && r.apiUuid && first.apiUuid === r.apiUuid;
        if (!sameApi) {
          setDiffError(first?.apiUuid || r.apiUuid ? "history.diffApiMismatch" : "history.diffNoApi");
          return prev;
        }
      }
      return [...prev, r.id];
    });
  }, [records]);

  /** 加载两个勾选记录的详情并比对 */
  const startDiff = useCallback(async () => {
    if (diffIds.length !== 2) return;
    setDiffLoading(true);
    setDiffError("");
    try {
      const [a, b] = await Promise.all([historyDetail(diffIds[0]), historyDetail(diffIds[1])]);
      setDiffPair({ a, b });
    } catch (e) {
      console.error(e);
      setDiffError(String(e));
    } finally {
      setDiffLoading(false);
    }
  }, [diffIds]);

  const exitDiff = useCallback(() => {
    setDiffPair(null);
    setDiffMode(false);
    setDiffIds([]);
  }, []);

  const totalCount = days.reduce((s, d) => s + d.count, 0);

  return {
    records,
    days,
    loading,
    hasMore,
    loaded,
    selectedId,
    detail,
    detailLoading,
    totalCount,
    offset,
    loadPage,
    reload,
    select,
    clearAll,
    diffMode,
    diffIds,
    diffPair,
    diffLoading,
    diffError,
    toggleDiffMode,
    toggleDiffSelect,
    startDiff,
    exitDiff,
  };
}
