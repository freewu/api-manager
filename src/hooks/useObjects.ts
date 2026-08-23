import { useCallback, useEffect, useState } from "react";
import { ObjectImportResult, ObjectStore, ObjectUsageItem } from "../types";
import { importDdl, importJsonObject, listObjects, objectUsage, saveObjects } from "../commands";

/** 对象管理：加载 / 保存 / JSON 导入 / 引用统计 */
export function useObjects(workspace: string | null) {
  const [store, setStore] = useState<ObjectStore>({ groups: [], objects: [] });
  const [usage, setUsage] = useState<ObjectUsageItem[]>([]);
  const [loaded, setLoaded] = useState(false);

  const refresh = useCallback(async () => {
    const s = await listObjects();
    setStore(s);
    try {
      setUsage(await objectUsage(s));
    } catch {
      setUsage([]);
    }
    setLoaded(true);
  }, []);

  useEffect(() => {
    if (workspace) void refresh();
  }, [workspace, refresh]);

  const save = useCallback(
    async (s: ObjectStore): Promise<ObjectStore> => {
      await saveObjects(s);
      // 回读后端权威数据（目录扫描 + hash 重算），保证前端与磁盘一致
      const fresh = await listObjects();
      setStore(fresh);
      try {
        setUsage(await objectUsage(fresh));
      } catch {
        setUsage([]);
      }
      return fresh;
    },
    []
  );

  const doImport = useCallback(
    async (name: string, group: string, json: string): Promise<ObjectImportResult> => {
      const res = await importJsonObject(name, group, json);
      const s = await listObjects();
      setStore(s);
      try {
        setUsage(await objectUsage(s));
      } catch {
        setUsage([]);
      }
      return res;
    },
    []
  );

  const doImportDdl = useCallback(
    async (group: string, ddl: string): Promise<ObjectImportResult> => {
      const res = await importDdl(group, ddl);
      const s = await listObjects();
      setStore(s);
      try {
        setUsage(await objectUsage(s));
      } catch {
        setUsage([]);
      }
      return res;
    },
    []
  );

  return { store, usage, loaded, refresh, save, doImport, doImportDdl };
}
