import { useState } from "react";
import { api, errorText, type CalibrationStep, type MicType, type Recommendation, type StepResult } from "../api";
import { formatDb, READ_PHRASE, warningText } from "../texts";
import { useCountdown } from "../useCountdown";
import { usePlayback } from "../usePlayback";
import { Help } from "./Help";

interface Props {
  ready: boolean;
  micType: MicType | null;
  recommendation: Recommendation | null;
  onRecommendation: (r: Recommendation | null) => void;
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
      return `Ruído de fundo: ${formatDb(r.noise_floor_dbfs, "dBFS")}`;
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
  if (g > 1) return `Sua voz está ${n} dB abaixo do ideal. O Sonora vai subir o volume em ${n} dB.`;
  if (g < -1) return `Sua voz está ${n} dB acima do ideal. O Sonora vai abaixar o volume em ${n} dB.`;
  return "Seu volume já está no ponto certo.";
}

/** Calibração smart: 3 testes curtos e uma recomendação em português claro. */
export function Calibration({ ready, micType, recommendation, onRecommendation, onError }: Props) {
  const [results, setResults] = useState<Partial<Record<CalibrationStep, StepResult>>>({});
  const [running, setRunning] = useState<CalibrationStep | null>(null);
  const [runningSecs, setRunningSecs] = useState<number | null>(null);
  const left = useCountdown(runningSecs);
  const { playing, play, stop } = usePlayback(onError);

  async function run(step: CalibrationStep, secs: number) {
    setRunning(step);
    setRunningSecs(secs);
    try {
      const r = await api.calibrateStep(step, secs);
      const next = { ...results, [step]: r };
      setResults(next);
      if (next.silence && next.speech) {
        onRecommendation(await api.calibrationResult());
      }
    } catch (e) {
      onError(errorText(e));
    } finally {
      setRunning(null);
      setRunningSecs(null);
    }
  }

  function reset() {
    setResults({});
    onRecommendation(null);
  }

  return (
    <section className="card">
      <header className="card-head">
        <h2>Calibração smart</h2>
        <Help>
          São três testes curtos. Com eles o Sonora descobre quanto volume falta, onde fica o
          barulho do seu quarto e se a sua voz estoura. Refaça sempre que mudar de quarto, de
          microfone ou de posição.
        </Help>
      </header>

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

      {recommendation && (
        <div className="recommendation">
          <h3>Resultado</h3>
          <p className="big">{gainSentence(recommendation.gain_db)}</p>

          {recommendation.warnings.length > 0 && (
            <ul className="warnings">
              {recommendation.warnings.map((w) => (
                <li key={w}>{warningText(w, micType)}</li>
              ))}
            </ul>
          )}

          <dl className="numbers">
            <div>
              <dt>
                Ajuste de volume <Help>Quanto o Sonora sobe ou desce a sua voz para chegar em -18 LUFS, o volume ideal de conversa.</Help>
              </dt>
              <dd>{recommendation.gain_db > 0 ? "+" : ""}{formatDb(recommendation.gain_db, "dB")}</dd>
            </div>
            <div>
              <dt>
                Voz × barulho <Help>A distância entre a sua voz e o barulho de fundo. Acima de 30 dB é ótimo; abaixo de 25 dB o barulho aparece.</Help>
              </dt>
              <dd>{formatDb(recommendation.voice_to_noise_db, "dB")}</dd>
            </div>
            <div>
              <dt>
                Porta de ruído <Help>Abaixo deste nível o microfone fica mudo, para ninguém ouvir ventilador ou teclado entre as suas frases.</Help>
              </dt>
              <dd>{formatDb(recommendation.gate_threshold_dbfs, "dBFS")}</dd>
            </div>
          </dl>

          <div className="row">
            <button className="secondary" disabled={playing !== null} onClick={() => play(null)}>
              {playing === "raw" ? "Tocando…" : "▶ Ouvir como estava"}
            </button>
            <button disabled={playing !== null} onClick={() => play(recommendation.gain_db)}>
              {playing === "processed" ? "Tocando…" : "▶ Ouvir com o ajuste do Sonora"}
            </button>
            {playing && (
              <button className="secondary" onClick={stop}>
                ■ Parar
              </button>
            )}
          </div>
          <p className="note">
            Use fone de ouvido para comparar. Nesta versão o Sonora mostra o ajuste; na próxima, ele
            aplica sozinho no Discord. Enquanto isso, você pode subir o volume do microfone no
            Windows: Configurações › Sistema › Som › seu microfone › Volume de entrada.
          </p>
          <button className="link" onClick={reset}>
            Recomeçar a calibração
          </button>
        </div>
      )}
    </section>
  );
}
