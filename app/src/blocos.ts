// Marca cada linha do capítulo com o que o editor precisa para desenhá-la
// como o .docx: o estilo nomeado em que ela está, se é título e se é um
// marcador `:::`. Função pura, sem navegador, para ser testada sozinha.

import { tituloDaLinha } from "./texto.ts";

export interface MarcaDeLinha {
  /** Nome do estilo de `::: {custom-style="Nome"}` que envolve a linha. */
  estilo: string | null;
  /** Linha de título (`# ...`) para o Pandoc, que não a formata como corpo. */
  titulo: boolean;
  /** A própria linha `:::` que abre ou fecha um bloco. */
  marcador: boolean;
  /** Linha de um bloco de código cercado, inclusive as duas cercas. */
  codigo: boolean;
  /** A linha em branco que separa dois parágrafos: a linha vazia logo depois
   *  de uma linha com texto, quando ainda há linha depois dela. O editor a
   *  esconde, para os parágrafos ficarem juntos como no Word. */
  separador: boolean;
}

const FECHAMENTO = /^:{3,}\s*$/;
const ABERTURA = /^:{3,}\s*(\S.*)$/;
const ESTILO = /custom-style=(?:"([^"]+)"|'([^']+)')/;

const CERCA_DE_CODIGO = /^ {0,3}(`{3,}|~{3,})/;

/**
 * Uma marca por linha. Os blocos `:::` do Pandoc aninham: cada abertura
 * empilha o seu estilo (ou nada, se não tem `custom-style`) e cada
 * fechamento desempilha. A linha fica com o estilo mais interno.
 *
 * Dentro de um bloco de código cercado (crases ou til) nada é interpretado,
 * como no Pandoc: o `:::` ali é texto.
 *
 * Título é o que o Pandoc lê como título (a mesma regra de `tituloDoTexto`):
 * colado num parágrafo, sem linha em branco antes, o `# ...` é texto dele.
 */
export function marcarLinhas(linhas: readonly string[]): MarcaDeLinha[] {
  const pilha: (string | null)[] = [];
  const nada: MarcaDeLinha = { estilo: null, titulo: false, marcador: false, codigo: false, separador: false };
  const codigo: MarcaDeLinha = { ...nada, codigo: true };
  /** A cerca que abriu o bloco de código em que estamos. */
  let cerca: string | null = null;
  /** A linha de cima é texto de parágrafo. */
  let emParagrafo = false;
  const marcas = linhas.map((linha): MarcaDeLinha => {
    const marcaDeCerca = CERCA_DE_CODIGO.exec(linha)?.[1];
    if (cerca !== null) {
      // Fecha com o mesmo caractere, no mínimo do mesmo tamanho, e mais nada.
      const fecha =
        marcaDeCerca !== undefined &&
        marcaDeCerca[0] === cerca[0] &&
        marcaDeCerca.length >= cerca.length &&
        linha.trim() === marcaDeCerca;
      if (fecha) cerca = null;
      return { ...codigo };
    }
    if (marcaDeCerca !== undefined) {
      cerca = marcaDeCerca;
      emParagrafo = false;
      return { ...codigo };
    }
    if (FECHAMENTO.test(linha)) {
      // Fechamento sem bloco aberto é texto comum.
      if (pilha.length === 0) {
        emParagrafo = true;
        return { ...nada };
      }
      pilha.pop();
      emParagrafo = false;
      return { ...nada, marcador: true };
    }
    const abertura = ABERTURA.exec(linha);
    if (abertura) {
      const estilo = ESTILO.exec(abertura[1] ?? "");
      pilha.push(estilo?.[1] ?? estilo?.[2] ?? null);
      emParagrafo = false;
      return { ...nada, marcador: true };
    }
    const estilo = pilha.findLast((nome) => nome !== null) ?? null;
    const titulo = tituloDaLinha(linha) !== null && !emParagrafo;
    if (!titulo) emParagrafo = linha.trim() !== "";
    return { ...nada, estilo, titulo };
  });
  // A separadora depende da linha de cima e de haver linha embaixo: vai numa
  // segunda passada, com todas as marcas prontas.
  return marcas.map((marca, indice) => ({
    ...marca,
    separador:
      linhas[indice] === "" && !marca.codigo && indice > 0 && linhas[indice - 1] !== "" && indice < linhas.length - 1,
  }));
}
