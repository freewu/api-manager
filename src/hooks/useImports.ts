import { useState } from "react";
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
import type { ImportResultView } from "../components/ImportResultModal";

/**
 * 全部导入格式的统一入口：调用后端导入 → 刷新树 → 展示导入结果弹窗 → 热重载 Mock。
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
  /** 导入结果弹窗数据（null = 未打开） */
  const [result, setResult] = useState<ImportResultView | null>(null);

  const finish = async (toastKey: string, vars?: Record<string, string | number>) => {
    if (!workspace) return;
    await loadAll(workspace);
    onToast(t(toastKey, vars));
    void reloadMockIfRunning(mockRunning);
  };

  const fail = (e: unknown) => onToast(t("toast.importFailed", { err: String(e) }));

  /** 归一化为导入结果视图并打开查看弹窗 */
  const showResult = (r: { folder: string; http?: number; ws?: number; graphql?: number; socketio?: number; objects?: number; failed?: number; duplicated?: number }) => {
    setResult({
      folder: r.folder,
      http: r.http ?? 0,
      ws: r.ws ?? 0,
      graphql: r.graphql ?? 0,
      socketio: r.socketio ?? 0,
      objects: r.objects ?? 0,
      failed: r.failed ?? 0,
      duplicated: r.duplicated ?? 0,
    });
  };

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
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportOpenApi = async () => {
    try {
      const result = await importOpenApi();
      if (!result) return;
      await finish("toast.importedOpenApi", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportApifox = async () => {
    try {
      const result = await importApifox();
      if (!result) return;
      await finish("toast.importedApifox", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportApipost = async () => {
    try {
      const result = await importApipost();
      if (!result) return;
      await finish("toast.importedApipost", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportRaml = async () => {
    try {
      const result = await importRaml();
      if (!result) return;
      await finish("toast.importedRaml", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportWadl = async () => {
    try {
      const result = await importWadl();
      if (!result) return;
      await finish("toast.importedWadl", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportHar = async () => {
    try {
      const result = await importHar();
      if (!result) return;
      await finish("toast.importedHar", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportYapi = async () => {
    try {
      const result = await importYapi();
      if (!result) return;
      await finish("toast.importedYapi", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportEolink = async () => {
    try {
      const result = await importEolink();
      if (!result) return;
      await finish("toast.importedEolink", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportInsomnia = async () => {
    try {
      const result = await importInsomnia();
      if (!result) return;
      await finish("toast.importedInsomnia", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportJmeter = async () => {
    try {
      const result = await importJmeter();
      if (!result) return;
      await finish("toast.importedJmeter", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportApiDoc = async () => {
    try {
      const result = await importApiDoc();
      if (!result) return;
      await finish("toast.importedApiDoc", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportMarkdown = async () => {
    try {
      const result = await importMarkdown();
      if (!result) return;
      await finish("toast.importedMarkdown", { count: result.count });
      showResult(result);
    } catch (e) {
      fail(e);
    }
  };

  const handleImportExtra = async (format: string) => {
    try {
      const result = await importExtra(format);
      if (!result) return;
      await finish("toast.importedExtra", { count: result.count });
      showResult(result);
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
    importResult: result,
    closeImportResult: () => setResult(null),
  };
}
