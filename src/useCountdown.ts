import { useEffect, useState } from "react";

/** Contagem regressiva visual enquanto o Rust grava. */
export function useCountdown(totalSecs: number | null) {
  const [left, setLeft] = useState(0);
  useEffect(() => {
    if (totalSecs === null) return;
    const start = performance.now();
    setLeft(totalSecs);
    const id = window.setInterval(() => {
      const l = Math.max(0, totalSecs - (performance.now() - start) / 1000);
      setLeft(l);
      if (l <= 0) window.clearInterval(id);
    }, 100);
    return () => window.clearInterval(id);
  }, [totalSecs]);
  return left;
}
