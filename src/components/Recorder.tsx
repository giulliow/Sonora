import { useState } from "react";
import { api, errorText, type TakeSummary } from "../api";
import { formatDb } from "../texts";
import { useCountdown } from "../useCountdown";
import { usePlayback } from "../usePlayback";
import { Help } from "./Help";

const SECS = 10;

/** Gravar e ouvir de volta: o "retorno real" de como você soa. */
export function Recorder({ ready, onError }: { ready: boolean; onError: (msg: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [take, setTake] = useState<TakeSummary | null>(null);
  const left = useCountdown(recording ? SECS : null);
  const { playing, play, stop } = usePlayback(onError);
  const busy = recording || playing !== null;

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
          Grave {SECS} segundos falando normalmente e ouça de volta no fone. Com o tratamento do
          Windows ligado, a gravação já sai exatamente como seus amigos recebem. Sem ele, dá para
          ouvir uma prévia de como vai ficar.
        </Help>
      </header>
      <div className="row">
        <button disabled={!ready || busy} onClick={record}>
          {recording ? `● Gravando… ${Math.ceil(left)} s` : take ? "● Gravar de novo" : `● Gravar ${SECS} s`}
        </button>
        <button className="secondary" disabled={!take || busy} onClick={() => play("raw")}>
          {playing === "raw" ? "Tocando…" : "▶ Ouvir"}
        </button>
        {take && !take.processed_by_windows && (
          <button className="secondary" disabled={busy} onClick={() => play("preview")}>
            {playing === "preview" ? "Tocando…" : "▶ Ouvir com o Sonora (prévia)"}
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
          {take.processed_by_windows ? "Gravado com o tratamento do Windows. " : "Gravado sem tratamento. "}
          Volume {formatDb(take.loudness_lufs, "LUFS")}, pico {formatDb(take.peak_dbfs, "dBFS")}.
        </p>
      )}
    </section>
  );
}
