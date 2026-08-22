import { useState } from "react";
import { vcsCommitPush, vcsSync } from "../commands";

/**
 * 工作目录版本控制（git / svn）：状态检测、同步、提交并推送。
 */
export function useVcs(opts: {
  syncRemote: boolean;
  onToast: (msg: string) => void;
  onNotify: (n: { title: string; body: string }) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
}) {
  const [vcs, setVcs] = useState<"git" | "svn" | null>(null);
  const { syncRemote, onToast, onNotify, t } = opts;

  /** 同步（git pull / svn update） */
  const handleVcsSync = async () => {
    if (!vcs || !syncRemote) return; // 按钮仅在开启同步远程时显示
    try {
      const out = await vcsSync(syncRemote);
      onToast(out.split("\n")[0] || t("toast.synced"));
    } catch (e) {
      onNotify({ title: t("notify.syncFailed"), body: String(e) });
    }
  };

  /** 提交并 Push 远程（未开启同步远程时只提交） */
  const handleVcsCommitPush = async () => {
    if (!vcs) return;
    try {
      const out = await vcsCommitPush(syncRemote);
      onToast(out.split("\n")[0] || t("toast.committed"));
    } catch (e) {
      onNotify({ title: t("notify.commitFailed"), body: String(e) });
    }
  };

  return { vcs, setVcs, handleVcsSync, handleVcsCommitPush };
}
