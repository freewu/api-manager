import { useCallback, useState } from "react";
import { mockReload, saveEnv, updateTrayEnv } from "../commands";
import { EnvStore, EnvVariable, MockStatus, emptyEnv } from "../types";

/**
 * 环境变量集：当前激活环境、切换/保存、主页面直接编辑变量值。
 */
export function useEnvs(opts: {
  mockRunning: boolean;
  onMockReloaded: (s: MockStatus) => void;
  onToast: (msg: string) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}) {
  const [envs, setEnvs] = useState<EnvStore>(emptyEnv());
  const [envModal, setEnvModal] = useState(false);
  const [envValue, setEnvValue] = useState(false);

  const { mockRunning, onMockReloaded, onToast, t } = opts;

  const switchActive = useCallback(
    async (active: string) => {
      const next = { ...envs, active };
      setEnvs(next);
      updateTrayEnv(active || "").catch(() => {});
      try {
        await saveEnv(next);
        if (mockRunning) onMockReloaded(await mockReload());
        onToast(active ? t("toast.envSwitched", { name: active }) : t("toast.noEnv"));
      } catch (e) {
        onToast(t("toast.envSwitchFailed", { err: String(e) }));
      }
    },
    [envs, mockRunning, onMockReloaded, onToast, t]
  );

  const save = useCallback(
    async (data: EnvStore) => {
      try {
        await saveEnv(data);
        setEnvs(data);
        updateTrayEnv(data.active || "").catch(() => {});
        setEnvModal(false);
        if (mockRunning) onMockReloaded(await mockReload());
        onToast(data.active ? t("toast.envSaved", { name: data.active }) : t("toast.envSavedNone"));
      } catch (e) {
        onToast(t("toast.saveEnvFailed", { err: String(e) }));
      }
    },
    [mockRunning, onMockReloaded, onToast, t]
  );

  // 主页面直接编辑当前环境集的变量值
  const activeEnv = envs.environments.find((e) => e.name === envs.active);
  const saveValues = useCallback(
    async (variables: EnvVariable[]) => {
      if (!activeEnv) return;
      const next: EnvStore = {
        ...envs,
        environments: envs.environments.map((e) =>
          e.name === activeEnv.name ? { ...e, variables } : e
        ),
      };
      setEnvs(next);
      try {
        await saveEnv(next);
        updateTrayEnv(next.active || "").catch(() => {});
        if (mockRunning) onMockReloaded(await mockReload());
        onToast(t("toast.envValuesSaved", { name: activeEnv.name }));
      } catch (e) {
        onToast(t("toast.saveEnvValuesFailed", { err: String(e) }));
      }
      setEnvValue(false);
    },
    [envs, activeEnv, mockRunning, onMockReloaded, onToast, t]
  );

  /** 加载工作区后同步环境变量集（loadAll 内调用） */
  const hydrate = useCallback((data: EnvStore | null) => {
    const envData = data || emptyEnv();
    setEnvs(envData);
    updateTrayEnv(envData.active || "").catch(() => {});
  }, []);

  return { envs, envModal, setEnvModal, envValue, setEnvValue, activeEnv, switchActive, save, saveValues, hydrate };
}
