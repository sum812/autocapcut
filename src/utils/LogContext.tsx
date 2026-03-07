import {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";

interface LogCtx {
  logs: string[];
  clearLogs: () => void;
  startListening: () => Promise<() => void>;
}

const LogContext = createContext<LogCtx | null>(null);

export function LogProvider({ children }: { children: ReactNode }) {
  const [logs, setLogs] = useState<string[]>([]);

  const clearLogs = useCallback(() => setLogs([]), []);

  const startListening = useCallback(async () => {
    const unlisten = await listen<string>("log", (event) => {
      setLogs((prev) => [...prev.slice(-999), event.payload]);
    });
    return unlisten;
  }, []);

  return (
    <LogContext.Provider value={{ logs, clearLogs, startListening }}>
      {children}
    </LogContext.Provider>
  );
}

export function useLogs() {
  const ctx = useContext(LogContext);
  if (!ctx) throw new Error("useLogs must be used within LogProvider");
  return ctx;
}
