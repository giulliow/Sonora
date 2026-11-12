import { useState } from "react";
import { api, errorText, type TakeSummary } from "../api";
import { formatDb } from "../texts";
import { useCountdown } from "../useCountdown";
import { usePlayback } from "../usePlayback";
import { Help } from "./Help";

const SECS = 10;

/** Gravar e ouvir de volta: o "retorno real" de como você soa. */
export function Recorder({
  ready,
  gainDb,
  onError,
}: {
  ready: boolean;
  gainDb: number | null;
  onError: (msg: string) => void;
}) {
  const [recording, setRecording] = useState(false);
  const [take, setTake] = useState<TakeSummary | null>(null);
  const left = useCountdown(recording ? SECS : null);
  const { playing, play, stop } = usePlayback(onError);

  async function record() {
    setRecording(true);
    try {
      setTake(await api.recordTake(SECS));
    } catch (e) {
      onError(errorText(e));
    } finally {
      setRecording(false);
    }
  }

  return (
    <section className="card">
      <header className="card-head">
        <h2>Gravar e ouvir</h2>
        <Help>
          Grave {SECS} segundos falando normalmente e ouça de volta no fone. É exatamente o que
          seus amigos recebem. Depois da calibração, dá para comparar com o ajuste do Sonora.
        </Help>
      </header>
      <div className="row">
        <button disabled={!ready || recording || playing !== null} onClick={record}>
          {recording ? `● Gravando… ${Math.ceil(left)} s` : take ? "● Gravar de novo" : `● Gravar ${SECS} s`}
        </button>
        <button className="secondary" disabled={!take || recording || playing !== null} onClick={() => play(null)}>
          {playing === "raw" ? "Tocando…" : "▶ Ouvir"}
        </button>
        {gainDb !== null && (
          <button className="secondary" disabled={!take || recording || playing !== null} onClick={() => play(gainDb)}>
            {playing === "processed" ? "Tocando…" : "▶ Ouvir com ajuste"}
          </button>
        )}
        {playing && (
          <button className="secondary" onClick={stop}>
            ■ Parar
          </button>
        )}
      </div>
      {take && (
        <p className="step-result">
          Gravação de {take.seconds.toFixed(0)} s: volume {formatDb(take.loudness_lufs, "LUFS")}, pico{" "}
          {formatDb(take.peak_dbfs, "dBFS")}.
        </p>
      )}
    </section>
  );
}
