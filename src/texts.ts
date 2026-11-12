// Todos os textos explicativos do app num lugar só, em linguagem simples.
import type { ChainParams, MicType, Warning, Zone } from "./api";

export const MIC_TYPES: Record<
  MicType,
  { label: string; example: string; description: string; tips: string[] }
> = {
  dynamic: {
    label: "Dinâmico",
    example: "Shure MV7, SM7B, Samson Q2U",
    description:
      "Pega pouco barulho do quarto, mas precisa de mais volume e de você falar bem perto.",
    tips: [
      "Fale na ponta do microfone (a grade da frente), não por cima dele.",
      "Fique a uns 3 a 8 cm da boca. Longe disso ele fica baixo, mesmo com volume alto.",
    ],
  },
  condenser: {
    label: "Condensador",
    example: "Blue Yeti, HyperX QuadCast, Fifine",
    description:
      "Muito sensível e detalhado, mas pega teclado, ventilador e o eco do quarto.",
    tips: [
      "Fique a uns 15 a 20 cm, de frente para o lado certo do microfone.",
      "Afaste o microfone do teclado e do PC, se puder.",
    ],
  },
  headset: {
    label: "Headset",
    example: "Fones gamer com haste, AirPods",
    description: "O microfone fica na haste do fone. Prático, mas costuma soar abafado.",
    tips: [
      "Deixe a haste no canto da boca, não na frente (evita o som do sopro).",
    ],
  },
  built_in: {
    label: "Embutido",
    example: "Microfone do notebook ou da webcam",
    description: "Fica longe da boca e pega tudo em volta. É o mais difícil de deixar bom.",
    tips: ["Chegue mais perto da tela ou da webcam.", "Evite digitar enquanto fala."],
  },
  lavalier: {
    label: "Lapela",
    example: "Microfones de prender na roupa",
    description: "Fica preso na roupa, perto do peito. Pode roçar no tecido.",
    tips: ["Prenda a uns 20 cm abaixo do queixo, longe de colares e zíperes."],
  },
};

export const ZONE_TEXT: Record<Zone, { label: string; detail: string }> = {
  low: {
    label: "Baixo",
    detail: "Seus amigos vão ter dificuldade para te ouvir.",
  },
  good: { label: "Bom", detail: "Volume confortável para conversa." },
  hot: {
    label: "Alto",
    detail: "Ainda não distorce, mas está no limite. Se gritar, pode estourar.",
  },
  clip: {
    label: "Estourou",
    detail: "Bateu no máximo e distorceu. Afaste-se um pouco ou baixe o volume do microfone.",
  },
};

export function warningText(w: Warning, mic: MicType | null): string {
  switch (w) {
    case "no_speech_detected":
      return "Não ouvimos sua voz no teste. Confira se o microfone certo está escolhido e se ele não está mudo.";
    case "too_far_from_mic":
      return mic === "dynamic"
        ? "Você parece longe do microfone. Microfone dinâmico precisa de você bem perto: fale na ponta dele, a uns 3 a 8 cm."
        : "Você parece longe do microfone. Chegue mais perto e repita o teste.";
    case "needs_more_input_gain":
      return "Falta muito volume. Dá para compensar no app, mas o ruído sobe junto. Melhor subir o volume do próprio microfone (no Windows ou no botão do mic) e repetir o teste.";
    case "noisy_room":
      return "O barulho de fundo está perto do volume da sua voz. A porta de ruído e a supressão de ruído vão ajudar, mas desligar ventilador ou fechar a janela ajuda mais.";
    case "input_clipping":
      return "Sua voz alta estourou antes de chegar no app, e isso não tem conserto depois. Baixe um pouco o volume do microfone ou se afaste quando for gritar.";
  }
}

export const GLOSSARY: { term: string; text: string }[] = [
  {
    term: "Pico (dBFS)",
    text: "O ponto mais alto do som. 0 é o teto: passou disso, distorce. Voz boa fica com picos entre -12 e -3.",
  },
  {
    term: "Volume percebido (LUFS)",
    text: "O quão alto as pessoas acham que você está, do jeito que o ouvido escuta. Para conversa no Discord, o ideal é por volta de -18.",
  },
  {
    term: "Ganho",
    text: "O \"volume\" do microfone antes de tudo. Pouco ganho, você fica baixo; demais, a voz estoura e chia.",
  },
  {
    term: "Filtro de graves",
    text: "Corta sons muito graves que não são voz: batida na mesa, ar-condicionado, tranco no braço do microfone.",
  },
  {
    term: "Supressão de ruído",
    text: "Tira barulho de fundo contínuo, como ventilador, teclado e rua, mantendo a voz.",
  },
  {
    term: "Porta de ruído",
    text: "Deixa o microfone mudo quando você não está falando e abre quando você fala. Se cortar o começo das frases, ela está alta demais.",
  },
  {
    term: "Equalizador",
    text: "Ajusta o timbre: mais grave dá corpo à voz, mais agudo dá clareza.",
  },
  {
    term: "De-esser",
    text: "Suaviza os \"S\" e \"X\" que ficam chiados, principalmente em microfones condensadores.",
  },
  {
    term: "Compressor",
    text: "Deixa o volume mais uniforme: sobe as partes baixas e segura as altas. É o que faz a voz soar \"de rádio\".",
  },
  {
    term: "Limitador",
    text: "Uma \"parede\" que nunca deixa o som passar do máximo. Protege o ouvido dos seus amigos quando você grita.",
  },
];

export const READ_PHRASE =
  "Oi, pessoal! Estou testando meu microfone novo. Dá para me ouvir bem? Hoje à noite a gente joga umas partidas e depois vê aquele filme.";

export function formatDb(v: number | null | undefined, unit: string): string {
  if (v === null || v === undefined || v <= -99) return "—";
  return `${v.toFixed(1).replace(".", ",")} ${unit}`;
}

// ------------------------------------------------------------ controles


type NumKey = { [K in keyof ChainParams]: ChainParams[K] extends number ? K : never }[keyof ChainParams];
type FlagKey = { [K in keyof ChainParams]: ChainParams[K] extends boolean ? K : never }[keyof ChainParams];

export interface FieldSpec {
  key: NumKey;
  label: string;
  help: string;
  min: number;
  max: number;
  step: number;
  unit: string;
}

export interface EffectSpec {
  id: string;
  title: string;
  what: string;
  when: string;
  toggle: FlagKey | null;
  fields: FieldSpec[];
}

/** O modo avançado: cada efeito, na ordem em que a voz passa por eles. */
export const EFFECTS: EffectSpec[] = [
  {
    id: "gain",
    title: "Ganho de entrada",
    what: "O \"volume\" do microfone antes de tudo.",
    when: "A calibração acerta. Mexa se pedirem mais (ou menos) volume.",
    toggle: null,
    fields: [
      { key: "input_gain_db", label: "Ganho", help: "Quanto subir (+) ou baixar (-) a voz.", min: -20, max: 40, step: 0.5, unit: "dB" },
    ],
  },
  {
    id: "hpf",
    title: "Filtro de graves",
    what: "Corta sons muito graves que não são voz: batida na mesa, ar-condicionado, tranco no braço do microfone.",
    when: "Deixe ligado. Suba a frequência se ainda ouvir \"tum\" quando encosta na mesa.",
    toggle: "hpf_on",
    fields: [
      { key: "hpf_hz", label: "Corta abaixo de", help: "Voz masculina grave começa por volta de 80 Hz; acima de 150 Hz a voz pode afinar.", min: 40, max: 250, step: 5, unit: "Hz" },
    ],
  },
  {
    id: "denoise",
    title: "Supressão de ruído (IA)",
    what: "Tira barulho de fundo contínuo, como ventilador, teclado e rua, mantendo a voz.",
    when: "Ligue em quarto barulhento. Desligue se a voz ficar \"robótica\".",
    toggle: "denoise_on",
    fields: [],
  },
  {
    id: "gate",
    title: "Porta de ruído",
    what: "Abaixa o microfone quando você não está falando e abre quando você fala.",
    when: "Se ouvirem sua respiração ou o teclado nos silêncios, suba o limiar. Se cortar o começo das frases, abaixe.",
    toggle: "gate_on",
    fields: [
      { key: "gate_threshold_db", label: "Abre a partir de", help: "Nível em que a porta abre. Mais alto corta mais, mas pode comer palavras baixas.", min: -90, max: -10, step: 1, unit: "dBFS" },
      { key: "gate_range_db", label: "Quanto abaixa fechada", help: "-80 é quase mudo; -15 só abaixa um pouco e soa mais natural.", min: -80, max: 0, step: 1, unit: "dB" },
      { key: "gate_hold_ms", label: "Espera para fechar", help: "Tempo que a porta fica aberta depois que você para de falar.", min: 0, max: 1000, step: 10, unit: "ms" },
      { key: "gate_release_ms", label: "Velocidade para fechar", help: "Mais alto fecha devagar e suave; mais baixo fecha seco.", min: 10, max: 1000, step: 10, unit: "ms" },
    ],
  },
  {
    id: "eq",
    title: "Equalizador",
    what: "Ajusta o timbre: graves dão corpo, médios-agudos dão clareza, agudos dão brilho.",
    when: "Voz abafada: mais clareza. Voz fina: mais corpo. Chiado: menos brilho.",
    toggle: "eq_on",
    fields: [
      { key: "eq_low_db", label: "Corpo (150 Hz)", help: "Graves da voz. Demais deixa \"embolado\".", min: -12, max: 12, step: 0.5, unit: "dB" },
      { key: "eq_presence_db", label: "Clareza (3,5 kHz)", help: "Faz a voz \"furar\" o som do jogo. Demais fica estridente.", min: -12, max: 12, step: 0.5, unit: "dB" },
      { key: "eq_air_db", label: "Brilho (10 kHz)", help: "O \"ar\" da voz. Demais realça chiado e \"S\".", min: -12, max: 12, step: 0.5, unit: "dB" },
    ],
  },
  {
    id: "deess",
    title: "De-esser",
    what: "Suaviza os \"S\" e \"X\" que ficam chiados.",
    when: "Ligue se os \"S\" machucarem o ouvido, comum em condensadores.",
    toggle: "deess_on",
    fields: [
      { key: "deess_threshold_db", label: "Age a partir de", help: "Mais baixo pega mais \"S\".", min: -60, max: 0, step: 1, unit: "dBFS" },
      { key: "deess_max_reduction_db", label: "Abaixa no máximo", help: "Demais deixa a voz com \"língua presa\".", min: 0, max: 20, step: 0.5, unit: "dB" },
    ],
  },
  {
    id: "comp",
    title: "Compressor",
    what: "Deixa o volume uniforme: sobe as partes baixas e segura as altas. É o que faz a voz soar \"de rádio\".",
    when: "Mais forte se você oscila entre sussurro e grito.",
    toggle: "comp_on",
    fields: [
      { key: "comp_threshold_db", label: "Começa a agir em", help: "Acima deste nível o compressor segura a voz.", min: -50, max: 0, step: 1, unit: "dBFS" },
      { key: "comp_ratio", label: "Força", help: "2 é leve, 4 é firme, 8 ou mais é quase um limitador.", min: 1, max: 10, step: 0.1, unit: ":1" },
      { key: "comp_attack_ms", label: "Reação", help: "Quão rápido segura um pico. Rápido demais tira a \"pegada\" da voz.", min: 0.5, max: 100, step: 0.5, unit: "ms" },
      { key: "comp_release_ms", label: "Soltura", help: "Quão rápido solta depois do pico. Rápido demais faz o volume \"bombear\".", min: 20, max: 1000, step: 10, unit: "ms" },
      { key: "comp_makeup_db", label: "Volume depois", help: "Recupera o volume que o compressor tirou.", min: -6, max: 20, step: 0.5, unit: "dB" },
    ],
  },
  {
    id: "limiter",
    title: "Limitador",
    what: "Uma \"parede\" que nunca deixa o som passar do máximo. Protege o ouvido dos seus amigos quando você grita.",
    when: "Deixe sempre ligado.",
    toggle: "limiter_on",
    fields: [
      { key: "limiter_ceiling_db", label: "Teto", help: "-1 dBFS é o padrão seguro.", min: -6, max: 0, step: 0.1, unit: "dBFS" },
    ],
  },
];

export const LEVELS = ["Desligada", "Leve", "Média", "Forte"];

export const SIMPLE_TEXT = {
  volume: {
    title: "Volume",
    help: "Seus amigos te ouvem mais alto ou mais baixo. Soma ao ajuste da calibração.",
  },
  cleanup: {
    title: "Limpeza",
    help: "Quanto barulho de fundo tirar. Leve: filtro de graves e porta suave. Média: + supressão de ruído por IA. Forte: tudo no máximo, para quartos barulhentos.",
  },
  tone: {
    title: "Timbre",
    help: "Voz mais encorpada (graves) ou mais clara (agudos).",
  },
  evenness: {
    title: "Uniformidade",
    help: "Deixa sussurro e grito no mesmo volume, com o compressor. Forte é bom para quem ri ou grita jogando.",
  },
};

export function formatSigned(v: number, unit: string): string {
  const s = (v > 0 ? "+" : "") + v.toFixed(1).replace(".", ",");
  return `${s} ${unit}`;
}
