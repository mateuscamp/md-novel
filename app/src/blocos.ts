// Marca cada linha do capítulo com o que o editor precisa para desenhá-la
// como o .docx: o estilo nomeado em que ela está, se é título e se é um
// marcador `:::`. Função pura, sem navegador, para ser testada sozinha.

export interface MarcaDeLinha {
  /** Nome do estilo de `::: {custom-style="Nome"}` que envolve a linha. */
  estilo: string | null;
  /** Linha de título (`# ...`), que o Pandoc não formata como corpo. */
  titulo: boolean;
  /** A própria linha `:::` que abre ou fecha um bloco. */
  marcador: boolean;
}

const FECHAMENTO = /^:{3,}\s*$/;
const ABERTURA = /^:{3,}\s*(\S.*)$/;
const ESTILO = /custom-style=(?:"([^"]+)"|'([^']+)')/;
const TITULO = /^#{1,6}\s+\S/;

const CERCA_DE_CODIGO = /^ {0,3}(`{3,}|~{3,})/;

/**
 * Uma marca por linha. Os blocos `:::` do Pandoc aninham: cada abertura
 * empilha o seu estilo (ou nada, se não tem `custom-style`) e cada
 * fechamento desempilha. A linha fica com o estilo mais interno.
 *
 * Dentro de um bloco de código cercado (crases ou til) nada é interpretado,
 * como no Pandoc: o `:::` ali é texto.
 */
export function marcarLinhas(linhas: readonly string[]): MarcaDeLinha[] {
  const pilha: (string | null)[] = [];
  const nada: MarcaDeLinha = { estilo: null, titulo: false, marcador: false };
  /** A cerca que abriu o bloco de código em que estamos. */
  let cerca: string | null = null;
  return linhas.map((linha) => {
    const marcaDeCerca = CERCA_DE_CODIGO.exec(linha)?.[1];
    if (cerca !== null) {
      // Fecha com o mesmo caractere, no mínimo do mesmo tamanho, e mais nada.
      const fecha =
        marcaDeCerca !== undefined &&
        marcaDeCerca[0] === cerca[0] &&
        marcaDeCerca.length >= cerca.length &&
        linha.trim() === marcaDeCerca;
      if (fecha) cerca = null;
      return { ...nada };
    }
    if (marcaDeCerca !== undefined) {
      cerca = marcaDeCerca;
      return { ...nada };
    }
    if (FECHAMENTO.test(linha)) {
      // Fechamento sem bloco aberto é texto comum.
      if (pilha.length === 0) return { ...nada };
      pilha.pop();
      return { estilo: null, titulo: false, marcador: true };
    }
    const abertura = ABERTURA.exec(linha);
    if (abertura) {
      const estilo = ESTILO.exec(abertura[1] ?? "");
      pilha.push(estilo?.[1] ?? estilo?.[2] ?? null);
      return { estilo: null, titulo: false, marcador: true };
    }
    const estilo = pilha.findLast((nome) => nome !== null) ?? null;
    return { estilo, titulo: TITULO.test(linha), marcador: false };
  });
}
