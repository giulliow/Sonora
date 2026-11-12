import { useCallback, useEffect, useState } from "react";
import { api, errorText, type InputDevice, type MeterSnapshot, type MicType, type Recommendation } from "./api";
import { Calibration } from "./components/Calibration";
import { Glossary } from "./components/Glossary";
import { LevelMeter } from "./components/LevelMeter";
import { MicPicker } from "./components/MicPicker";
import { Recorder } from "./components/Recorder";

const micTypeKey = (name: string) => `sonora.micType.${name}`;

function loadMicType(device: InputDevice | undefined): MicType | null {
  if (!device) return null;
  try {
    const saved = localStorage.getItem(micTypeKey(device.name)) as MicType | null;
    if (saved) return saved;
  } catch {
    /* armazenamento indisponível: segue com o palpite */
  }
  return device.guess?.mic_type ?? null;
}

export default function App() {
  const [devices, setDevices] = useState<InputDevice[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [meter, setMeter] = useState<MeterSnapshot | null>(null);
  const [micType, setMicType] = useState<MicType | null>(null);
  const [recommendation, setRecommendation] = useState<Recommendation | null>(null);
  const [error, setError] = useState<string | null>(null);

  const showError = useCallback((msg: string) => setError(msg), []);

  const open = useCallback(async (name: string, list: InputDevice[]) => {
    setReady(false);
    setError(null);
    setRecommendation(null);
    setSelected(name);
    setMicType(loadMicType(list.find((d) => d.name === name)));
    try {
      await api.startInput(name);
      setReady(true);
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  const refresh = useCallback(async () => {
    try {
      const list = await api.listInputs();
      setDevices(list);
      const keep = list.find((d) => d.name === selected) ?? list.find((d) => d.is_default) ?? list[0];
      if (keep) await open(keep.name, list);
      else setError("Nenhum microfone encontrado. Conecte o microfone e clique em \"Procurar de novo\".");
    } catch (e) {
      setError(errorText(e));
    }
  }, [open, selected]);

  useEffect(() => {
    const unlisten = api.onMeter(setMeter);
    refresh();
    return () => {
      unlisten.then((f) => f());
    };
    // Só na abertura do app.
  }, []);

  function chooseMicType(t: MicType) {
    setMicType(t);
    if (selected) {
      try {
        localStorage.setItem(micTypeKey(selected), t);
      } catch {
        /* sem problema: só não lembra na próxima vez */
      }
    }
  }

  return (
    <main>
      <header className="app-head">
        <h1>Sonora</h1>
        <p>Deixe seu microfone soando bem, sem precisar entender de áudio.</p>
      </header>

      {error && (
        <div className="error" role="alert">
          <p>{error}</p>
          <p className="note">
            Se o Windows estiver bloqueando: Configurações › Privacidade e segurança › Microfone ›
            ligue "Permitir que aplicativos da área de trabalho acessem o microfone".
          </p>
          <button className="link" onClick={() => setError(null)}>
            Fechar
          </button>
        </div>
      )}

      <MicPicker
        devices={devices}
        selected={selected}
        onSelect={(n) => open(n, devices)}
        onRefresh={refresh}
        micType={micType}
        onMicType={chooseMicType}
      />
      <LevelMeter meter={ready ? meter : null} />
      <Calibration
        ready={ready}
        micType={micType}
        recommendation={recommendation}
        onRecommendation={setRecommendation}
        onError={showError}
      />
      <Recorder ready={ready} gainDb={recommendation?.gain_db ?? null} onError={showError} />
      <Glossary />
    </main>
  );
}
