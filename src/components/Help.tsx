import type { ReactNode } from "react";

/** Botão "?" que abre uma explicação curta. */
export function Help({ children, label = "O que é isso?" }: { children: ReactNode; label?: string }) {
  return (
    <details className="help">
      <summary aria-label={label}>?</summary>
      <div className="help-body">{children}</div>
    </details>
  );
}
