// A lista do livro como a lateral e a barra de status mostram: nome de
// exibição de cada capítulo, contagem de palavras e o total. Regra pura.

import { contarPalavras, tituloDoTexto } from "./texto.ts";

/** Um capítulo na lista. É o `CapituloNaLista` do Rust (`sessao.rs`). */
export interface CapituloNaLista {
  nome: string;
  /** O primeiro título Markdown do texto, se houver. */
  titulo: string | null;
  /** `null` quando o arquivo não pôde ser lido como capítulo. */
  palavras: number | null;
}

/** O título do capítulo; sem título, o nome do arquivo sem a extensão. */
export function nomeDeExibicao(capitulo: CapituloNaLista): string {
  return capitulo.titulo ?? (capitulo.nome.replace(/\.md$/i, "") || capitulo.nome);
}

/** As palavras do livro: a soma dos capítulos que deu para ler. */
export function totalDePalavras(capitulos: readonly CapituloNaLista[]): number {
  return capitulos.reduce((soma, capitulo) => soma + (capitulo.palavras ?? 0), 0);
}

/** Inteiro com ponto de milhar. Escrito à mão para não depender dos dados de
 *  idioma do navegador, que o WebKit de cada sistema traz de um jeito. */
export function numero(valor: number): string {
  return String(valor).replace(/\B(?=(\d{3})+$)/g, ".");
}

export function rotuloDePalavras(palavras: number | null): string {
  if (palavras === null) return "—";
  return `${numero(palavras)} ${palavras === 1 ? "palavra" : "palavras"}`;
}

/** A lista com o capítulo aberto contado pelo texto que está na tela. */
export function comTextoAoVivo(
  capitulos: readonly CapituloNaLista[],
  nome: string,
  texto: string,
): CapituloNaLista[] {
  return capitulos.map((capitulo) =>
    capitulo.nome === nome
      ? { nome, titulo: tituloDoTexto(texto), palavras: contarPalavras(texto) }
      : capitulo,
  );
}
