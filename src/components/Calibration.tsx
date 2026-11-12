import { useState } from "react";
import {
  api,
  errorText,
  type CalibrationOutcome,
  type CalibrationStep,
  type MicType,
  type StepResult,
} from "../api";
import { formatDb, formatSigned, LEVELS, READ_PHRASE, warningText } from "../texts";
import { useCountdown } from "../useCountdown";
import { usePlayback } from "../usePlayback";
import { Help } from "./Help";

interface Props {
  ready: boolean;
  micType: MicType | null;
  windowsActive: boolean;
  onUse: (outcome: CalibrationOutcome) => void;
  onError: (msg: string) => void;
}

const STEPS: { id: CalibrationStep; secs: number; title: string; instruction: string }[] = [
  {
    id: "silence",
    secs: 5,
    title: "1. Silêncio",
    instruction: "Fique em silêncio, sem digitar. Vamos medir o barulho de fundo do seu quarto.",
  },
  {
    id: "speech",
    secs: 10,
    title: "2. Voz normal",
    instruction: "Leia a frase abaixo como se estivesse falando com um amigo no Discord.",
  },
  {
    id: "loud",
    secs: 5,
    title: "3. Voz alta",
    instruction: "Agora dê uma risada ou fale alto, como numa jogada emocionante.",
  },
];

function stepSummary(r: StepResult): string {
  switch (r.step) {
    case "silence":
      return r.noise_floor_dbfs <= -90
        ? "Silêncio total: seu microfone já corta o ruído sozinho nas pausas."
        : `Ruído de fundo: ${formatDb(r.noise_floor_dbfs, "dBFS")}`;
    case "speech":
      return `Sua voz: ${formatDb(r.loudness_lufs, "LUFS")} (pico ${formatDb(r.peak_dbfs, "dBFS")})`;
    case "loud":
      return r.clipped
        ? `Pico: ${formatDb(r.peak_dbfs, "dBFS")}. Estourou!`
        : `Pico: ${formatDb(r.peak_dbfs, "dBFS")}. Sem estourar.`;
  }
}

function gainSentence(g: number): string {
  const n = Math.abs(g).toFixed(1).replace(".", ",");
  if (g > 1) return `Sua voz está ${n} dB abaixo do ideal.`;
  if (g < -1) return `Sua voz está ${n} dB acima do ideal.`;
  return "Seu volume já está no ponto certo.";
}

/** Calibração smart: 3 testes curtos. O resultado é só uma sugestão. */
export function Calibration({ ready, micType, windowsActive, onUse, onError }: Props) {
  const [results, setResults] = useState<Partial<Record<CalibrationStep, StepResult>>>({});
  const [outcome, setOutcome] = useState<CalibrationOutcome | null>(null);
  const [used, setUsed] = useState(false);
  const [running, setRunning] = useState<CalibrationStep | null>(null);
  const [runningSecs, setRunningSecs] = useState<number | null>(null);
  const left = useCountdown(runningSecs);
  const { playing, play, stop } = usePlayback(onError);

  async function run(step: CalibrationStep, secs: number) {
    setRunning(step);
    setRunningSecs(secs);
    setUsed(false);
    try {
      const r = await api.calibrateStep(step, secs);
      const next = { ...results, [step]: r };
      setResults(next);
      if (next.silence && next.speech) setOutcome(await api.calibrationResult());
    } catch (e) {
      onError(errorText(e));
    } finally {
      setRunning(null);
      setRunningSecs(null);
    }
  }

  function use() {
    if (!outcome) return;
    onUse(outcome);
    setUsed(true);
  }

  const rec = outcome?.recommendation;
  const sug = outcome?.suggested_simple;

  return (
    <section className="card">
      <header className="card-head">
        <h2>Calibração smart</h2>
        <Help>
          São três testes curtos. Com eles o Sonora descobre quanto volume falta, o barulho do seu
          quarto e se a sua voz estoura. O resultado é só uma sugestão: você decide se usa, e pode
          mudar tudo depois. Refaça quando mudar de quarto, de microfone ou de posição.
        </Help>
      </header>
      {windowsActive && (
        <p className="note">
          Durante os testes o tratamento do Windows é desligado por alguns segundos, para medir o
          microfone puro. Depois ele volta sozinho.
        </p>
      )}

      <ol className="steps">
        {STEPS.map((s, i) => {
          const done = results[s.id];
          const blocked = !ready || running !== null || (i > 0 && !results[STEPS[i - 1].id]);
          const isRunning = running === s.id;
          return (
            <li key={s.id} className={`step ${done ? "done" : ""} ${isRunning ? "running" : ""}`}>
              <div className="step-head">
                <strong>{s.title}</strong>
                <button disabled={blocked} onClick={() => run(s.id, s.secs)}>
                  {isRunning ? `${Math.ceil(left)} s…` : done ? "Refazer" : "Começar"}
                </button>
              </div>
              <p>{s.instruction}</p>
              {s.id === "speech" && <blockquote>{READ_PHRASE}</blockquote>}
              {isRunning && (
                <div className="progress">
                  <span style={{ width: `${100 - (left / s.secs) * 100}%` }} />
                </div>
              )}
              {done && <p className="step-result">{stepSummary(done)}</p>}
            </li>
          );
        })}
      </ol>

      {rec && sug && (
        <div className="recommendation">
          <h3>Sugestão do Sonora</h3>
          <p className="big">{gainSentence(rec.gain_db)}</p>

          {rec.warnings.length > 0 && (
            <ul className="warnings">
              {rec.warnings.map((w) => (
                <li key={w}>{warningText(w, micType)}</li>
              ))}
            </ul>
          )}

          <dl className="numbers">
            <div>
              <dt>Volume</dt>
              <dd>{formatSigned(rec.gain_db, "dB")}</dd>
            </div>
            <div>
              <dt>Limpeza</dt>
              <dd>{LEVELS[sug.cleanup]}</dd>
            </div>
            <div>
              <dt>Uniformidade</dt>
              <dd>{LEVELS[sug.evenness]}</dd>
            </div>
            <div>
              <dt>
                Porta de ruído{" "}
                <Help>
                  {rec.mic_mutes_itself
                    ? "Seu microfone já fica mudo sozinho nas pausas, então a porta de ruído do Sonora não é necessária."
                    : "Abaixo deste nível o microfone fica mudo, para ninguém ouvir ventilador ou teclado entre as suas frases."}
                </Help>
              </dt>
              <dd>{rec.mic_mutes_itself ? "Não precisa" : formatDb(rec.gate_threshold_dbfs, "dBFS")}</dd>
            </div>
          </dl>
          {rec.loud_peak_after_gain_dbfs !== null && rec.loud_peak_after_gain_dbfs > 0 && (
            <p className="note">
              Com esse volume, seus gritos passariam do máximo. Por isso a sugestão é Uniformidade
              forte (compressor) e o limitador ligado: a fala normal sobe e os gritos ficam seguros.
            </p>
          )}

          <div className="row">
            <button onClick={use} disabled={used}>
              {used ? "✓ Calibração em uso" : "Usar esta calibração"}
            </button>
            <button className="secondary" disabled={playing !== null} onClick={() => play("raw")}>
              {playing === "raw" ? "Tocando…" : "▶ Ouvir como estava"}
            </button>
            <button className="secondary" disabled={playing !== null || !used} onClick={() => play("preview")}>
              {playing === "preview" ? "Tocando…" : "▶ Ouvir com o Sonora"}
            </button>
            {playing && (
              <button className="secondary" onClick={stop}>
                ■ Parar
              </button>
            )}
          </div>
          <p className="note">
            "Usar esta calibração" troca os controles abaixo pela sugestão. Depois você pode mudar o que
            quiser. Use fone de ouvido para comparar.
          </p>
        </div>
      )}
    </section>
  );
}
