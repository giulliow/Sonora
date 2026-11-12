import type { InputDevice, MicType } from "../api";
import { MIC_TYPES } from "../texts";
import { Help } from "./Help";

interface Props {
  devices: InputDevice[];
  selected: string | null;
  onSelect: (name: string) => void;
  onRefresh: () => void;
  micType: MicType | null;
  onMicType: (t: MicType) => void;
}

const ORDER: MicType[] = ["dynamic", "condenser", "headset", "built_in", "lavalier"];

/** Escolher o microfone e confirmar (ou informar) o tipo dele. */
export function MicPicker({ devices, selected, onSelect, onRefresh, micType, onMicType }: Props) {
  const device = devices.find((d) => d.name === selected) ?? null;
  const guess = device?.guess ?? null;
  const recognized = guess?.confidence === "high" && guess.mic_type === micType;

  return (
    <section className="card">
      <header className="card-head">
        <h2>Seu microfone</h2>
        <Help>
          Escolha o microfone que você usa para falar. Se não aparecer, conecte ele e clique em
          "Procurar de novo".
        </Help>
      </header>

      <div className="row">
        <select
          value={selected ?? ""}
          onChange={(e) => onSelect(e.target.value)}
          aria-label="Microfone"
        >
          {devices.length === 0 && <option value="">Nenhum microfone encontrado</option>}
          {devices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name}
              {d.is_default ? " (padrão do sistema)" : ""}
            </option>
          ))}
        </select>
        <button className="secondary" onClick={onRefresh}>
          Procurar de novo
        </button>
      </div>

      {recognized ? (
        <p className="recognized">
          Reconhecemos o <strong>{guess!.model}</strong>: microfone{" "}
          {MIC_TYPES[micType!].label.toLowerCase()}. {MIC_TYPES[micType!].description}
        </p>
      ) : (
        <p className="ask">
          {guess
            ? "Parece ser este tipo de microfone. Confira se está certo:"
            : "Não reconhecemos esse microfone. Qual destes é o seu?"}
        </p>
      )}

      <div className="mic-types" role="radiogroup" aria-label="Tipo de microfone">
        {ORDER.map((t) => (
          <button
            key={t}
            role="radio"
            aria-checked={micType === t}
            className={`mic-type ${micType === t ? "on" : ""}`}
            onClick={() => onMicType(t)}
          >
            <strong>{MIC_TYPES[t].label}</strong>
            <span>{MIC_TYPES[t].example}</span>
          </button>
        ))}
      </div>

      {micType && (
        <div className="tips">
          <h3>Dicas para o seu tipo de microfone</h3>
          <ul>
            {MIC_TYPES[micType].tips.map((t) => (
              <li key={t}>{t}</li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
