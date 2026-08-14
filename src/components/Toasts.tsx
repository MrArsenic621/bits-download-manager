import {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
} from "react";
import { AlertIcon, CheckCircleIcon } from "../lib/icons";

type ToastKind = "info" | "success" | "error";

interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

const ToastContext = createContext<{
  push: (message: string, kind?: ToastKind) => void;
}>({ push: () => {} });

export const useToast = () => useContext(ToastContext);

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const counter = useRef(0);

  const push = useCallback((message: string, kind: ToastKind = "info") => {
    const id = ++counter.current;
    setToasts((t) => [...t, { id, message, kind }]);
    setTimeout(() => {
      setToasts((t) => t.filter((x) => x.id !== id));
    }, 4200);
  }, []);

  return (
    <ToastContext.Provider value={{ push }}>
      {children}
      <div className="toasts">
        {toasts.map((t) => (
          <div key={t.id} className={`toast toast-${t.kind}`}>
            {t.kind === "error" ? (
              <AlertIcon width={15} height={15} />
            ) : t.kind === "success" ? (
              <CheckCircleIcon width={15} height={15} />
            ) : null}
            <span>{t.message}</span>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
