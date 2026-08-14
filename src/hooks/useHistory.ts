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

/**
 * 请求历史列表状态：分页懒加载（每页 100 条）+ 选中记录详情
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
    } catch (e) {
      console.error(e);
    }
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
  };
}
