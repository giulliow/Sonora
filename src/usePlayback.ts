import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorText } from "./api";

/** Tocar a última gravação e saber quando terminou. */
export function usePlayback(onError: (msg: string) => void) {
  const [playing, setPlaying] = useState<"raw" | "preview" | null>(null);
  const timer = useRef<number | null>(null);

  const clear = () => {
    if (timer.current !== null) window.clearInterval(timer.current);
    timer.current = null;
  };

  useEffect(() => clear, []);

  const play = useCallback(
    async (mode: "raw" | "preview") => {
      clear();
      try {
        await api.playTake(mode);
        setPlaying(mode);
        timer.current = window.setInterval(async () => {
          if (!(await api.isPlaying())) {
            clear();
            setPlaying(null);
          }
        }, 200);
      } catch (e) {
        setPlaying(null);
        onError(errorText(e));
      }
    },
    [onError],
  );

  const stop = useCallback(async () => {
    clear();
    setPlaying(null);
    await api.stopPlayback().catch(() => {});
  }, []);

  return { playing, play, stop };
}
