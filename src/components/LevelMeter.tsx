import type { MeterSnapshot } from "../api";
import { formatDb, ZONE_TEXT } from "../texts";
import { Help } from "./Help";

const MIN = -60;
const pct = (db: number) => Math.max(0, Math.min(100, ((db - MIN) / -MIN) * 100));

/** Barra de nível com zonas coloridas, como um medidor de estúdio simplificado. */
export function LevelMeter({ meter }: { meter: MeterSnapshot | null }) {
  const peak = meter?.peak_dbfs ?? -100;
  const silent = peak < -50;
  const zone = meter?.zone ?? "low";
  const status = silent
    ? { label: "Esperando você falar…", detail: "Fale normalmente para ver o medidor se mexer." }
    : ZONE_TEXT[zone];

  return (
    <section className="card">
      <header className="card-head">
        <h2>Medidor ao vivo</h2>
        <Help>
          A barra mostra o volume da sua voz agora. Fale normalmente: o ideal é ela ficar
          na faixa verde. Cinza é baixo demais, amarelo está no limite e vermelho estourou.
        </Help>
      </header>

      <div className="meter" role="meter" aria-valuemin={MIN} aria-valuemax={0} aria-valuenow={Math.round(peak)}>
        <div className="meter-zones">
          <span className="z-low" style={{ width: `${pct(-24)}%` }} />
          <span className="z-good" style={{ width: `${pct(-3) - pct(-24)}%` }} />
          <span className="z-hot" style={{ width: `${pct(-0.5) - pct(-3)}%` }} />
          <span className="z-clip" style={{ width: `${100 - pct(-0.5)}%` }} />
        </div>
        <div className="meter-fill" style={{ width: `${pct(peak)}%` }} />
        {meter && meter.hold_dbfs > MIN && (
          <div className="meter-hold" style={{ left: `${pct(meter.hold_dbfs)}%` }} />
        )}
      </div>
      <div className="meter-scale" aria-hidden>
        {[-60, -48, -36, -24, -12, -6, 0].map((d) => (
          <span key={d} style={{ left: `${pct(d)}%` }}>{d}</span>
        ))}
      </div>

      <p className={`status status-${silent ? "idle" : zone}`}>
        <strong>{status.label}.</strong> {status.detail}
        {meter?.recording && <span className="rec-dot"> ● gravando</span>}
      </p>

      <dl className="numbers">
        <div>
          <dt>Pico</dt>
          <dd>{formatDb(meter?.peak_dbfs, "dBFS")}</dd>
        </div>
        <div>
          <dt>Volume percebido (3 s)</dt>
          <dd>{formatDb(meter?.short_term_lufs, "LUFS")}</dd>
        </div>
      </dl>
    </section>
  );
}
