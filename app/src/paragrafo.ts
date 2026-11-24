// Parágrafos como no Word (ADR 0003): o Enter cria um parágrafo novo, e o
// app grava a linha em branco que o Markdown pede entre um e outro. Aqui
// fica a regra de quando isso vale, pura; quem liga é o `editor.ts`, que
// também deixa de fora os blocos de código.

const ITEM_DE_LISTA = /^\s*(?:[-*+]|\d+[.)])\s/;

/**
 * Com o cursor nesta linha, o Enter cria um parágrafo (duas quebras de
 * linha)? Não onde uma linha em branco estragaria o Markdown ou não faz
 * sentido: linha já em branco, o começo de um parágrafo, item de lista, citação, tabela, linha de
 * bloco `:::` e código recuado. Ali o Enter é o de sempre, uma quebra só.
 */
export function enterCriaParagrafo(
  linha: string,
  { noComeco = false, deCimaVazia = false }: { noComeco?: boolean; deCimaVazia?: boolean } = {},
): boolean {
  if (linha.trim() === "") return false;
  // No começo de um parágrafo que já tem a linha em branco em cima, uma
  // quebra só o empurra uma linha para baixo, como no Word; duas abririam
  // duas linhas em branco à vista.
  if (noComeco && deCimaVazia) return false;
  if (linha.trimStart().startsWith(":::")) return false;
  if (ITEM_DE_LISTA.test(linha)) return false;
  if (/^\s*>/.test(linha)) return false;
  if (linha.includes("|")) return false;
  if (/^(?: {4}|\t)/.test(linha)) return false;
  return true;
}
