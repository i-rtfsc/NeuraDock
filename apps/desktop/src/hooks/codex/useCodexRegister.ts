import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { events } from '@/lib/tauri';
import type { CodexRegisterProgress, RegisterConfig } from '@/lib/tauri';
import { useQueryClient } from '@tanstack/react-query';

export interface RegisterLogEntry {
  id: number;
  message: string;
  status?: CodexRegisterProgress['status'];
  timestamp: string;
}

export interface CurrentTaskInfo {
  taskId: string;
  email: string | null;
  status: CodexRegisterProgress['status'];
}

export function useCodexRegister() {
  const qc = useQueryClient();
  const [isRunning, setIsRunning] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [logs, setLogs] = useState<RegisterLogEntry[]>([]);
  const [progress, setProgress] = useState<CodexRegisterProgress | null>(null);
  const [currentTask, setCurrentTask] = useState<CurrentTaskInfo | null>(null);
  const logIdRef = useRef(0);
  const unlistenRef = useRef<(() => void) | null>(null);
  const cancelRequestedRef = useRef(false);
  const latestProgressRef = useRef<CodexRegisterProgress | null>(null);

  const addLog = useCallback((entry: Omit<RegisterLogEntry, 'id' | 'timestamp'>) => {
    setLogs((prev) => [
      ...prev,
      {
        ...entry,
        id: ++logIdRef.current,
        timestamp: new Date().toLocaleTimeString('zh-CN', { hour12: false }),
      },
    ]);
  }, []);

  const clearLogs = useCallback(() => {
    setLogs([]);
    setProgress(null);
    setCurrentTask(null);
    latestProgressRef.current = null;
  }, []);

  const startRegistration = useCallback(
    async (config: RegisterConfig) => {
      if (isRunning) return;

      setIsRunning(true);
      setIsCancelling(false);
      cancelRequestedRef.current = false;
      clearLogs();
      addLog({ message: '▶ 开始注册任务...', status: 'running' });

      try {
        const unlisten = await events.codexRegisterProgress.listen((event) => {
          const payload = event.payload;
          latestProgressRef.current = payload;
          setProgress(payload);

          setCurrentTask({
            taskId: payload.taskId,
            email: payload.email ?? null,
            status: payload.status,
          });

          addLog({ message: payload.message, status: payload.status });
        });
        unlistenRef.current = unlisten;

        await invoke('register_codex_accounts', { config });

        const finalProgress = latestProgressRef.current;

        if (cancelRequestedRef.current || finalProgress?.status === 'cancelled') {
          addLog({ message: '⛔ 注册任务已取消', status: 'cancelled' });
        } else if (finalProgress && finalProgress.current >= finalProgress.total) {
          if (finalProgress.failCount === 0 && finalProgress.successCount > 0) {
            addLog({ message: '✅ 注册任务全部完成', status: 'success' });
            qc.invalidateQueries({ queryKey: ['codex-accounts'] });
          } else if (finalProgress.successCount > 0) {
            addLog({
              message: `⚠ 注册任务已结束：成功 ${finalProgress.successCount}，失败 ${finalProgress.failCount}`,
              status: 'failed',
            });
            qc.invalidateQueries({ queryKey: ['codex-accounts'] });
          } else {
            addLog({ message: '❌ 注册任务已结束：全部失败', status: 'failed' });
          }
        } else if (!finalProgress) {
          addLog({ message: '❌ 注册任务异常中断：未收到进度事件', status: 'failed' });
        } else {
          addLog({ message: '❌ 注册任务异常中断：未到达终态', status: 'failed' });
        }
      } catch (e: any) {
        addLog({ message: `❌ 注册失败: ${e?.message ?? e}`, status: 'failed' });
      } finally {
        unlistenRef.current?.();
        unlistenRef.current = null;
        setIsCancelling(false);
        setIsRunning(false);
      }
    },
    [isRunning, addLog, clearLogs, qc]
  );

  const cancelRegistration = useCallback(async () => {
    if (!isRunning || isCancelling) return;

    setIsCancelling(true);
    cancelRequestedRef.current = true;
    addLog({ message: '⛔ 已请求取消注册，等待当前步骤结束...', status: 'cancelled' });

    try {
      await invoke('cancel_codex_registration');
    } catch (e: any) {
      cancelRequestedRef.current = false;
      setIsCancelling(false);
      addLog({ message: `❌ 取消失败: ${e?.message ?? e}`, status: 'failed' });
    }
  }, [addLog, isCancelling, isRunning]);

  return {
    isRunning,
    isCancelling,
    logs,
    progress,
    currentTask,
    startRegistration,
    cancelRegistration,
    clearLogs,
  };
}
