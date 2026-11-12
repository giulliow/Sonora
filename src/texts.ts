// Todos os textos explicativos do app num lugar só, em linguagem simples.
import type { MicType, Warning, Zone } from "./api";

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
