import { useCallback, useEffect, useState } from "react";
import { GenLogItem, listGenLogs } from "../commands";

/** 数据生成记录视图状态：列表加载 / 选中 */
export function useGenLogs() {
  const [records, setRecords] = useState<GenLogItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const items = await listGenLogs();
      setRecords(items);
      setSelectedId((s) => (s && items.some((i) => i.file === s) ? s : items[0]?.file ?? null));
    } catch {
      setRecords([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const select = useCallback((id: string) => setSelectedId(id), []);

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { records, loading, selectedId, select, reload };
}
