// Reabrir onde parou: o capítulo e a posição do cursor em cada livro. Fica
// no armazenamento da janela, fora da pasta do livro; aqui só a leitura e a
// escrita do registro, puras.

export interface Onde {
  capitulo: string;
  /** A posição do cursor no texto do capítulo. */
  posicao: number;
}

export function escreverOnde(capitulo: string, posicao: number): string {
  return JSON.stringify({ capitulo, posicao });
}

/** O lugar guardado, se o capítulo ainda está no livro. Um registro
 *  estragado não vale; uma posição estragada vira o começo do capítulo. */
export function lerOnde(guardado: string | null, capitulos: string[]): Onde | null {
  if (guardado === null) return null;
  let lido: unknown;
  try {
    lido = JSON.parse(guardado);
  } catch {
    return null;
  }
  if (typeof lido !== "object" || lido === null || Array.isArray(lido)) return null;
  const { capitulo, posicao } = lido as Record<string, unknown>;
  if (typeof capitulo !== "string" || !capitulos.includes(capitulo)) return null;
  const valida = typeof posicao === "number" && Number.isSafeInteger(posicao) && posicao >= 0;
  return { capitulo, posicao: valida ? posicao : 0 };
}
