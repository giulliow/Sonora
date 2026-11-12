import { useCallback, useEffect, useRef, useState } from "react";
import {
  api,
  errorText,
  type AppView,
  type CalibrationOutcome,
  type InputDevice,
  type MeterSnapshot,
  type MicType,
  type Settings,
} from "./api";
import { Calibration } from "./components/Calibration";
import { Controls } from "./components/Controls";
import { Glossary } from "./components/Glossary";
import { LevelMeter } from "./components/LevelMeter";
import { MicPicker } from "./components/MicPicker";
import { Recorder } from "./components/Recorder";
import { WindowsSetup } from "./components/WindowsSetup";

const SAVE_DELAY_MS = 300;

export default function App() {
  const [view, setView] = useState<AppView | null>(null);
  const [devices, setDevices] = useState<InputDevice[]>([]);
  const [ready, setReady] = useState(false);
  const [meter, setMeter] = useState<MeterSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const saveTimer = useRef<number | null>(null);
  const pending = useRef<Settings | null>(null);

  const showError = useCallback((msg: string) => setError(msg), []);

  /** Muda as configurações na tela na hora e salva (aplicando no Windows) logo em seguida. */
  const change = useCallback((patch: Partial<Settings>) => {
    setView((v) => {
      if (!v) return v;
      const settings = { ...v.settings, ...patch };
      pending.current = settings;
      if (saveTimer.current !== null) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(async () => {
        const s = pending.current;
        if (!s) return;
        try {
          const next = await api.saveSettings(s);
          // Só aceita a resposta se nada mudou enquanto salvava.
          if (pending.current === s) setView(next);
        } catch (e) {
          setError(errorText(e));
        }
      }, SAVE_DELAY_MS);
      return { ...v, settings };
    });
  }, []);

  const openDevice = useCallback(async (name: string) => {
    setReady(false);
    setError(null);
    try {
      await api.startInput(name);
      setReady(true);
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  const selectDevice = useCallback(
    (name: string, list: InputDevice[], current: Settings) => {
      const device = list.find((d) => d.name === name);
      const sameDevice = current.device_name === name;
      change({
        device_name: name,
        mic_type: sameDevice ? (current.mic_type ?? device?.guess?.mic_type ?? null) : (device?.guess?.mic_type ?? null),
        // Calibração é de um microfone específico.
        ...(sameDevice ? {} : { recommendation: null }),
      });
      openDevice(name);
    },
    [change, openDevice],
  );

  const refresh = useCallback(async () => {
    try {
      const [v, list] = await Promise.all([api.getState(), api.listInputs()]);
      setView(v);
      setDevices(list);
      const keep =
        list.find((d) => d.name === v.settings.device_name) ?? list.find((d) => d.is_default) ?? list[0];
      if (!keep) {
        setError('Nenhum microfone encontrado. Conecte o microfone e clique em "Procurar de novo".');
        return;
      }
      if (keep.name !== v.settings.device_name || v.settings.mic_type === null) {
        selectDevice(keep.name, list, v.settings);
      } else {
        openDevice(keep.name);
      }
    } catch (e) {
      setError(errorText(e));
    }
  }, [openDevice, selectDevice]);

  useEffect(() => {
    const unlisten = api.onMeter(setMeter);
    refresh();
    return () => {
      unlisten.then((f) => f());
    };
    // Só na abertura do app.
  }, []);

  function applyCalibration(o: CalibrationOutcome) {
    change({ recommendation: o.recommendation, use_calibration: true, simple: o.suggested_simple, mode: "simple" });
  }

  if (!view) {
    return (
      <main>
        <header className="app-head">
          <h1>Sonora</h1>
          <p>Carregando…</p>
        </header>
        {error && <div className="error" role="alert"><p>{error}</p></div>}
      </main>
    );
  }

  const windowsActive = view.eqapo.linked && view.eqapo.processing_on;

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
            Se o Windows estiver bloqueando o microfone: Configurações › Privacidade e segurança ›
            Microfone › ligue "Permitir que aplicativos da área de trabalho acessem o microfone".
          </p>
          <button className="link" onClick={() => setError(null)}>
            Fechar
          </button>
        </div>
      )}

      <MicPicker
        devices={devices}
        selected={view.settings.device_name}
        onSelect={(n) => selectDevice(n, devices, view.settings)}
        onRefresh={refresh}
        micType={view.settings.mic_type}
        onMicType={(t: MicType) => change({ mic_type: t })}
      />
      <WindowsSetup
        view={view}
        deviceName={view.settings.device_name}
        onView={setView}
        onChange={change}
        onRefresh={refresh}
        onError={showError}
      />
      <LevelMeter meter={ready ? meter : null} processed={windowsActive} />
      <Calibration
        ready={ready}
        micType={view.settings.mic_type}
        windowsActive={windowsActive}
        onUse={applyCalibration}
        onError={showError}
      />
      <Controls view={view} onChange={change} />
      <Recorder ready={ready} onError={showError} />
      <Glossary />
    </main>
  );
}
