// Aspas curvas e travessão enquanto se digita, como no novelWriter. Aqui fica
// a regra, pura: o que entra no lugar da tecla, dado o que já está na linha.
// Quem liga no editor (e deixa de fora os blocos de código) é o `editor.ts`.

export interface Troca {
  /** Quantos caracteres antes do cursor saem junto. */
  apagar: number;
  /** O que entra no lugar deles e da tecla digitada. */
  inserir: string;
}

/** Depois destes, uma aspa abre; depois de qualquer outra coisa, fecha. */
const ANTES_DE_ABRIR = /[\s([{—–“‘-]$/u;

/**
 * O que digitar `digitado` vira, com `antes` sendo o texto da linha até o
 * cursor. `null` deixa a tecla como veio.
 *
 * Nada é trocado onde o Markdown precisa do sinal reto: na linha de um bloco
 * `:::` (o `custom-style="..."` leva aspas retas), dentro de código em linha
 * e numa linha de tabela.
 */
export function substituicao(antes: string, digitado: string): Troca | null {
  if (digitado !== '"' && digitado !== "'" && digitado !== "-") return null;
  if (antes.trimStart().startsWith(":::")) return null;
  if (antes.includes("|")) return null;
  const crases = antes.split("`").length - 1;
  if (crases % 2 === 1) return null;

  if (digitado === "-") {
    // A linha só com o travessão que acabou de nascer de `--`: o terceiro
    // hífen é a quebra de cena (`---`), que o Pandoc precisa ler como tal.
    if (antes === "—") return { apagar: 1, inserir: "---" };
    // O começo de um comentário HTML (`<!--`) leva os dois hifens.
    if (antes.endsWith("<!-")) return null;
    if (antes.endsWith("-") && !antes.endsWith("--")) return { apagar: 1, inserir: "—" };
    return null;
  }
  // O sinal de ênfase colado na aspa (`*"fala"*`) não conta: vale o que vem
  // antes dele.
  const semEnfase = antes.replace(/[*_]+$/u, "");
  const abre = semEnfase === "" || ANTES_DE_ABRIR.test(semEnfase);
  if (digitado === '"') return { apagar: 0, inserir: abre ? "“" : "”" };
  return { apagar: 0, inserir: abre ? "‘" : "’" };
}
