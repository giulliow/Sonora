import { useState } from "react";
import { api, errorText, type AppView, type Settings } from "../api";
import { Help } from "./Help";

interface Props {
  view: AppView;
  deviceName: string | null;
  onView: (v: AppView) => void;
  onChange: (patch: Partial<Settings>) => void;
  onRefresh: () => void;
  onError: (msg: string) => void;
}

/** Liga o tratamento dentro do Windows (via Equalizer APO) e guia a instalação. */
export function WindowsSetup({ view, deviceName, onView, onChange, onRefresh, onError }: Props) {
  const st = view.eqapo;
  const [busy, setBusy] = useState(false);
  const mic = deviceName ?? "seu microfone";

  async function act(f: () => Promise<unknown>) {
    setBusy(true);
    try {
      await f();
    } catch (e) {
      onError(errorText(e));
    } finally {
      setBusy(false);
    }
  }

  const help = (
    <Help>
      O tratamento roda dentro do próprio sistema de áudio do Windows, preso ao microfone. Todos os
      programas recebem a voz tratada, mesmo com o Sonora fechado. Nada fica rodando à toa: o
      processamento só acontece quando algum programa está usando o microfone, e gasta cerca de 1% de
      um núcleo do processador. Para isso o Sonora usa o Equalizer APO, gratuito e de código aberto.
    </Help>
  );

  if (!st.installed) {
    return (
      <section className="card setup">
        <header className="card-head">
          <h2>Ligar no Windows</h2>
          {help}
        </header>
        <p>Para a sua voz sair tratada em todos os programas, falta instalar uma peça grátis, uma vez só:</p>
        <ol className="setup-steps">
          <li>
            <button onClick={() => act(api.openEqapoDownload)} disabled={busy}>
              Baixar o Equalizer APO
            </button>{" "}
            e rode o instalador.
          </li>
          <li>
            No fim da instalação abre uma janela com uma lista. Clique na aba <strong>Capture devices</strong>{" "}
            (dispositivos de gravação) e marque <strong>{mic}</strong>. Não precisa marcar caixas de som.
          </li>
          <li>Clique em OK e <strong>reinicie o PC</strong> quando ele pedir.</li>
          <li>Abra o Sonora de novo e clique em <strong>Ativar o Sonora no Windows</strong>.</li>
        </ol>
        <button className="secondary" onClick={onRefresh}>
          Já instalei, verificar de novo
        </button>
      </section>
    );
  }

  if (!st.linked) {
    return (
      <section className="card setup">
        <header className="card-head">
          <h2>Ligar no Windows</h2>
          {help}
        </header>
        <p>Equalizer APO encontrado. Falta só ligar o Sonora nele.</p>
        <div className="row">
          <button disabled={busy} onClick={() => act(async () => onView(await api.activateWindows()))}>
            Ativar o Sonora no Windows
          </button>
          {st.configurator && (
            <button className="secondary" disabled={busy} onClick={() => act(api.openEqapoConfigurator)}>
              Escolher o microfone no Equalizer APO
            </button>
          )}
        </div>
        <p className="note">
          Se ainda não marcou o {mic} na aba "Capture devices" do Equalizer APO, use o segundo botão,
          marque, clique em OK e reinicie o PC.
        </p>
      </section>
    );
  }

  return (
    <section className="card setup">
      <header className="card-head">
        <h2>Tratamento no Windows</h2>
        {help}
      </header>
      <label className="big-switch">
        <input type="checkbox" checked={view.settings.enabled} onChange={(e) => onChange({ enabled: e.target.checked })} />
        <span>{view.settings.enabled ? "Ligado: todos os programas recebem sua voz tratada" : "Desligado: o microfone passa sem mudanças"}</span>
      </label>
      {!st.plugin_found && (
        <p className="warn">O módulo do Sonora não foi encontrado na pasta do app. Reinstale o Sonora.</p>
      )}
      {view.apply_error && <p className="warn">{view.apply_error}</p>}
      {st.has_other_settings && (
        <p className="note">
          A configuração do Equalizer APO tem outros ajustes seus além do Sonora. Eles continuam lá e
          também podem valer para o microfone.
        </p>
      )}
      <p className="note">
        Para conferir: fale e veja o medidor, ou grave em "Gravar e ouvir". Se nada mudar ao ligar e
        desligar, abra{" "}
        {st.configurator ? (
          <button className="link" onClick={() => act(api.openEqapoConfigurator)}>
            o Equalizer APO
          </button>
        ) : (
          "o Configurator do Equalizer APO"
        )}
        , confira se o {mic} está marcado em "Capture devices" e reinicie o PC. No Discord, desligue a
        supressão de ruído e o controle automático de ganho, porque o Sonora já faz isso.
      </p>
    </section>
  );
}
