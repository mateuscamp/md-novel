// Buscar no livro inteiro (ADR 0003): o que a lateral diz sobre os achados e
// com que termo a busca abre. Quem procura nos capítulos é o Rust.

import { numero } from "./resumo.ts";

/** O maior trecho selecionado que vira termo de busca. */
const MAIOR_TERMO_DA_SELECAO = 100;

/** A linha acima da lista: quantos achados, em quantos capítulos. Com a
 *  busca cortada, `total` é o teto que o Rust devolveu. */
export function resumoDosAchados(total: number, capitulos: number, cortada: boolean): string {
  if (total === 0) return "Nada encontrado no livro.";
  const emCapitulos = `${numero(capitulos)} ${capitulos === 1 ? "capítulo" : "capítulos"}`;
  if (cortada) {
    return `Mais de ${numero(total)} achados: a lista mostra os primeiros ${numero(total)}, em ${emCapitulos}. Escreva mais letras para afinar.`;
  }
  return `${numero(total)} ${total === 1 ? "achado" : "achados"} em ${emCapitulos}`;
}

/** O termo com que a busca abre: o que está selecionado no capítulo, se for
 *  um trecho curto de uma linha só; senão, nada (fica o termo anterior). */
export function termoDaSelecao(selecao: string): string {
  const termo = selecao.trim();
  return termo.includes("\n") || termo.length > MAIOR_TERMO_DA_SELECAO ? "" : termo;
}

/** Um achado da busca no livro, com a linha em volta dele. */
export interface Achado {
  inicio: number;
  fim: number;
  antes: string;
  trecho: string;
  depois: string;
}

/** O texto do capítulo ainda tem o achado no lugar em que a busca o viu: a
 *  palavra e a vizinhança dela. Só a palavra não basta: depois de uma
 *  edição, a posição antiga pode cair em outra ocorrência igual. As
 *  reticências são do corte que o Rust fez na linha, não do texto. */
export function achadoAindaVale(texto: string, achado: Achado): boolean {
  const antes = achado.antes.startsWith("…") ? achado.antes.slice(1) : achado.antes;
  const depois = achado.depois.endsWith("…") ? achado.depois.slice(0, -1) : achado.depois;
  return (
    texto.slice(achado.inicio, achado.fim) === achado.trecho &&
    texto.slice(Math.max(0, achado.inicio - antes.length), achado.inicio) === antes &&
    texto.slice(achado.fim, achado.fim + depois.length) === depois
  );
}
