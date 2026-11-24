// O painel "Estilo": as regras do formulário, puras. O estilo chega do Rust
// na forma do `estilo.yaml`: o texto do livro (o corpo) com todas as
// propriedades, e cada estilo nomeado só com as que ele troca. O que falta
// num estilo nomeado é herdado do corpo, como num estilo "baseado em" do
// Word. Quem valida e grava é o Rust.

export type Alinhamento = "esquerda" | "centro" | "direita" | "justificado";

/** As propriedades de um parágrafo, com as chaves do `estilo.yaml`. */
export interface Campos {
  fonte: string;
  /** Medida com unidade, como no arquivo: `12pt`. */
  tamanho: string;
  entrelinha: number;
  alinhamento: Alinhamento;
  "recuo-primeira-linha": string;
  "espaco-antes": string;
  "espaco-depois": string;
  italico: boolean;
  negrito: boolean;
}

/** O que um estilo nomeado troca em relação ao texto do livro. */
export type CamposTrocados = Partial<Campos>;

export interface EstiloNoPainel {
  corpo: Campos;
  estilos: Record<string, CamposTrocados>;
  /** O livro já tem `estilo.yaml`; sem ele o painel mostra o modelo. */
  existe: boolean;
}

/** As propriedades como valem para o parágrafo: `null` é o texto do livro. */
export function camposDe(estilo: EstiloNoPainel, nome: string | null): Campos {
  return nome === null ? estilo.corpo : { ...estilo.corpo, ...estilo.estilos[nome] };
}

/** O estilo nomeado não troca este campo: vale o do texto do livro. */
export function herdado(estilo: EstiloNoPainel, nome: string | null, campo: keyof Campos): boolean {
  if (nome === null) return false;
  const trocados = estilo.estilos[nome];
  return trocados === undefined || !(campo in trocados);
}

/** O estilo com um campo mudado. Num estilo nomeado, o valor igual ao do
 *  texto do livro deixa de ser uma troca: o campo volta a herdar. */
export function comCampo<K extends keyof Campos>(
  estilo: EstiloNoPainel,
  nome: string | null,
  campo: K,
  valor: Campos[K],
): EstiloNoPainel {
  if (nome === null) return { ...estilo, corpo: { ...estilo.corpo, [campo]: valor } };
  const atuais = estilo.estilos[nome];
  if (atuais === undefined) return estilo;
  const trocados: CamposTrocados = { ...atuais };
  if (valor === estilo.corpo[campo]) delete trocados[campo];
  else trocados[campo] = valor;
  return { ...estilo, estilos: { ...estilo.estilos, [nome]: trocados } };
}

/** Um estilo nomeado novo, que começa herdando tudo do texto do livro. */
export function comEstiloNovo(estilo: EstiloNoPainel, nome: string): EstiloNoPainel {
  return { ...estilo, estilos: { ...estilo.estilos, [nome]: {} } };
}

export function semEstilo(estilo: EstiloNoPainel, nome: string): EstiloNoPainel {
  const estilos = { ...estilo.estilos };
  delete estilos[nome];
  return { ...estilo, estilos };
}

/** O nome digitado para um estilo novo, sem os espaços de fora; `null` se
 *  está vazio ou já existe (o Word não distingue maiúscula em nome de
 *  estilo). O resto das regras do nome é do Rust. */
export function nomeDeEstiloNovo(estilo: EstiloNoPainel, digitado: string): string | null {
  const nome = digitado.trim();
  if (nome === "") return null;
  const repetido = Object.keys(estilo.estilos).some((outro) => outro.toLowerCase() === nome.toLowerCase());
  return repetido ? null : nome;
}

/** `12pt` vira o número e a unidade que o campo mostra. */
export function partirMedida(texto: string): { numero: number; unidade: string } | null {
  const achado = /^(\d+(?:\.\d+)?)(pt|cm|mm|in)$/.exec(texto);
  if (achado === null) return null;
  const [, numero = "", unidade = ""] = achado;
  return { numero: Number(numero), unidade };
}

export function juntarMedida(numero: number, unidade: string): string {
  return `${numero}${unidade}`;
}

/** Fontes de texto comuns, para a lista do campo: só entram as que estão
 *  instaladas. O autor pode digitar qualquer outra. */
export const FONTES_SUGERIDAS = [
  "EB Garamond",
  "Cormorant Garamond",
  "Libre Baskerville",
  "Crimson Pro",
  "Crimson Text",
  "Lora",
  "Merriweather",
  "Source Serif 4",
  "Noto Serif",
  "Liberation Serif",
  "DejaVu Serif",
  "Times New Roman",
  "Georgia",
  "Palatino Linotype",
  "Book Antiqua",
  "Courier Prime",
  "Courier New",
  "Liberation Mono",
  "Noto Sans",
  "Liberation Sans",
  "Arial",
];
