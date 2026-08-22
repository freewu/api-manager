import {
  importApiDoc,
  importApifox,
  importApipost,
  importEolink,
  importExtra,
  importHar,
  importInsomnia,
  importJmeter,
  importMarkdown,
  importOpenApi,
  importPostman,
  importRaml,
  importWadl,
  importYapi,
} from "../commands";

/**
 * 全部导入格式的统一入口：调用后端导入 → 刷新树 → 提示 → 热重载 Mock。
 */
export function useImports(opts: {
  workspace: string | null;
  loadAll: (ws: string) => Promise<void>;
  mockRunning: boolean;
  reloadMockIfRunning: (running: boolean) => Promise<void>;
  onToast: (msg: string) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}) {
  const { workspace, loadAll, mockRunning, reloadMockIfRunning, onToast, t } = opts;

  const finish = async (toastKey: string, vars?: Record<string, string | number>) => {
    if (!workspace) return;
    await loadAll(workspace);
    onToast(t(toastKey, vars));
    void reloadMockIfRunning(mockRunning);
  };

  const fail = (e: unknown) => onToast(t("toast.importFailed", { err: String(e) }));

  const handleImportPostman = async () => {
    try {
      const result = await importPostman();
      if (!result) return; // 用户取消
      if (!workspace) return;
      await loadAll(workspace);
      if (result.vars > 0) {
        onToast(t("toast.importedPostman", { count: result.vars, env: result.env }));
      } else {
        onToast(t("toast.importedPostmanSimple"));
      }
      void reloadMockIfRunning(mockRunning);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportOpenApi = async () => {
    try {
      const result = await importOpenApi();
      if (!result) return;
      await finish("toast.importedOpenApi", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportApifox = async () => {
    try {
      const result = await importApifox();
      if (!result) return;
      await finish("toast.importedApifox", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportApipost = async () => {
    try {
      const result = await importApipost();
      if (!result) return;
      await finish("toast.importedApipost", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportRaml = async () => {
    try {
      const result = await importRaml();
      if (!result) return;
      await finish("toast.importedRaml", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportWadl = async () => {
    try {
      const result = await importWadl();
      if (!result) return;
      await finish("toast.importedWadl", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportHar = async () => {
    try {
      const result = await importHar();
      if (!result) return;
      await finish("toast.importedHar", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportYapi = async () => {
    try {
      const result = await importYapi();
      if (!result) return;
      await finish("toast.importedYapi", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportEolink = async () => {
    try {
      const result = await importEolink();
      if (!result) return;
      await finish("toast.importedEolink", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportInsomnia = async () => {
    try {
      const result = await importInsomnia();
      if (!result) return;
      await finish("toast.importedInsomnia", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportJmeter = async () => {
    try {
      const result = await importJmeter();
      if (!result) return;
      await finish("toast.importedJmeter", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportApiDoc = async () => {
    try {
      const result = await importApiDoc();
      if (!result) return;
      await finish("toast.importedApiDoc", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportMarkdown = async () => {
    try {
      const result = await importMarkdown();
      if (!result) return;
      await finish("toast.importedMarkdown", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  const handleImportExtra = async (format: string) => {
    try {
      const result = await importExtra(format);
      if (!result) return;
      await finish("toast.importedExtra", { count: result.count });
    } catch (e) {
      fail(e);
    }
  };

  return {
    handleImportPostman,
    handleImportOpenApi,
    handleImportApifox,
    handleImportApipost,
    handleImportRaml,
    handleImportWadl,
    handleImportHar,
    handleImportYapi,
    handleImportEolink,
    handleImportInsomnia,
    handleImportJmeter,
    handleImportApiDoc,
    handleImportMarkdown,
    handleImportExtra,
  };
}
