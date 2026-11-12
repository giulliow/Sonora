import type { AppView, ChainParams, Settings, SimpleControls } from "../api";
import { EFFECTS, formatSigned, LEVELS, SIMPLE_TEXT, type EffectSpec } from "../texts";
import { Help } from "./Help";

interface Props {
  view: AppView;
  onChange: (patch: Partial<Settings>) => void;
}

function Segmented({ value, onChange, label }: { value: number; onChange: (v: number) => void; label: string }) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {LEVELS.map((l, i) => (
        <button key={l} role="radio" aria-checked={value === i} className={value === i ? "on" : ""} onClick={() => onChange(i)}>
          {l}
        </button>
      ))}
    </div>
  );
}

function SimpleMode({ view, onChange }: Props) {
  const s = view.settings.simple;
  const set = (patch: Partial<SimpleControls>) => onChange({ simple: { ...s, ...patch } });
  const rec = view.settings.use_calibration ? view.settings.recommendation : null;
  const isSuggested = JSON.stringify(s) === JSON.stringify(view.suggested_simple);

  return (
    <div className="simple">
      <div className="control">
        <div className="control-head">
          <strong>{SIMPLE_TEXT.volume.title}</strong>
          <span className="value">{formatSigned(s.volume_db, "dB")}</span>
        </div>
        <input type="range" min={-12} max={12} step={0.5} value={s.volume_db} onChange={(e) => set({ volume_db: Number(e.target.value) })} aria-label="Volume" />
        <p className="note">
          {SIMPLE_TEXT.volume.help}{" "}
          {rec ? `Total aplicado: ${formatSigned(view.simple_params.input_gain_db, "dB")} (calibração ${formatSigned(rec.gain_db, "dB")}).` : "Faça a calibração para o Sonora acertar o volume base."}
        </p>
      </div>

      <div className="control">
        <div className="control-head">
          <strong>{SIMPLE_TEXT.cleanup.title}</strong>
        </div>
        <Segmented label="Limpeza" value={s.cleanup} onChange={(v) => set({ cleanup: v })} />
        <p className="note">{SIMPLE_TEXT.cleanup.help}</p>
      </div>

      <div className="control">
        <div className="control-head">
          <strong>{SIMPLE_TEXT.tone.title}</strong>
          <span className="value">{s.tone === 0 ? "Natural" : s.tone < 0 ? `Encorpada ${-s.tone}` : `Clara ${s.tone}`}</span>
        </div>
        <input type="range" min={-3} max={3} step={1} value={s.tone} onChange={(e) => set({ tone: Number(e.target.value) })} aria-label="Timbre" />
        <div className="range-labels" aria-hidden>
          <span>Mais encorpada</span>
          <span>Mais clara</span>
        </div>
        <p className="note">{SIMPLE_TEXT.tone.help}</p>
      </div>

      <div className="control">
        <div className="control-head">
          <strong>{SIMPLE_TEXT.evenness.title}</strong>
        </div>
        <Segmented label="Uniformidade" value={s.evenness} onChange={(v) => set({ evenness: v })} />
        <p className="note">{SIMPLE_TEXT.evenness.help}</p>
      </div>

      <div className="row">
        <button className="secondary" disabled={isSuggested} onClick={() => onChange({ simple: view.suggested_simple })}>
          {isSuggested ? "✓ No recomendado" : "Voltar ao recomendado"}
        </button>
        {view.settings.recommendation && (
          <label className="check">
            <input type="checkbox" checked={view.settings.use_calibration} onChange={(e) => onChange({ use_calibration: e.target.checked })} />
            Usar a calibração ({formatSigned(view.settings.recommendation.gain_db, "dB")})
          </label>
        )}
      </div>
    </div>
  );
}

function Effect({ spec, params, defaults, set }: { spec: EffectSpec; params: ChainParams; defaults: ChainParams; set: (p: Partial<ChainParams>) => void }) {
  const on = spec.toggle ? params[spec.toggle] : true;
  return (
    <div className={`effect ${on ? "" : "off"}`}>
      <div className="effect-head">
        {spec.toggle ? (
          <label className="switch">
            <input type="checkbox" checked={on} onChange={(e) => set({ [spec.toggle!]: e.target.checked } as Partial<ChainParams>)} />
            <strong>{spec.title}</strong>
          </label>
        ) : (
          <strong>{spec.title}</strong>
        )}
        <Help>
          <p><strong>Para que serve:</strong> {spec.what}</p>
          <p><strong>Quando mexer:</strong> {spec.when}</p>
        </Help>
      </div>
      {spec.fields.map((f) => {
        const v = params[f.key];
        const d = defaults[f.key];
        const changed = Math.abs(v - d) > 1e-6;
        return (
          <div key={f.key} className="field">
            <div className="control-head">
              <span>
                {f.label} <Help>{f.help}</Help>
              </span>
              <span className="value">
                <input
                  type="number"
                  min={f.min}
                  max={f.max}
                  step={f.step}
                  value={Number(v.toFixed(2))}
                  onChange={(e) => {
                    const n = Number(e.target.value);
                    if (Number.isFinite(n)) set({ [f.key]: n } as Partial<ChainParams>);
                  }}
                  disabled={!on}
                  aria-label={f.label}
                />{" "}
                {f.unit}
                {changed && (
                  <button className="link small" onClick={() => set({ [f.key]: d } as Partial<ChainParams>)} title={`Recomendado: ${d}`}>
                    recomendado
                  </button>
                )}
              </span>
            </div>
            <input type="range" min={f.min} max={f.max} step={f.step} value={v} disabled={!on} onChange={(e) => set({ [f.key]: Number(e.target.value) } as Partial<ChainParams>)} aria-label={f.label} />
          </div>
        );
      })}
    </div>
  );
}

function AdvancedMode({ view, onChange }: Props) {
  const params = view.settings.advanced ?? view.simple_params;
  const set = (patch: Partial<ChainParams>) => onChange({ advanced: { ...params, ...patch } });
  return (
    <div className="advanced">
      <p className="note">
        A voz passa pelos efeitos nesta ordem. Cada um tem o "?" explicando para que serve. O link
        "recomendado" aparece quando um número está diferente da sugestão do Sonora.
      </p>
      {EFFECTS.map((spec) => (
        <Effect key={spec.id} spec={spec} params={params} defaults={view.simple_params} set={set} />
      ))}
      <div className="row">
        <button className="secondary" onClick={() => onChange({ advanced: view.simple_params })}>
          Voltar tudo ao recomendado
        </button>
      </div>
    </div>
  );
}

/** Ajustes: modo simples (4 controles) ou avançado (todos os números). */
export function Controls({ view, onChange }: Props) {
  const mode = view.settings.mode;
  return (
    <section className="card">
      <header className="card-head">
        <h2>Ajustes</h2>
        <Help>
          O modo simples tem 4 controles que mexem em vários efeitos de uma vez, do jeito certo. O modo
          avançado mostra todos os números. Tudo que você muda é salvo e, com o Sonora ativo no Windows,
          vale na hora.
        </Help>
        <div className="tabs" role="tablist">
          <button role="tab" aria-selected={mode === "simple"} className={mode === "simple" ? "on" : ""} onClick={() => onChange({ mode: "simple" })}>
            Simples
          </button>
          <button
            role="tab"
            aria-selected={mode === "advanced"}
            className={mode === "advanced" ? "on" : ""}
            onClick={() => onChange({ mode: "advanced", advanced: view.settings.advanced ?? view.simple_params })}
          >
            Avançado
          </button>
        </div>
      </header>
      {mode === "simple" ? <SimpleMode view={view} onChange={onChange} /> : <AdvancedMode view={view} onChange={onChange} />}
    </section>
  );
}
