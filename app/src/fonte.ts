// Duas perguntas sobre a fonte do livro que só o navegador responde.

const AMOSTRA = "mmmmmmmmmmlliWW@#áçõ0123456789";
const GENERICAS = ["monospace", "serif", "sans-serif"];

/**
 * A fonte está instalada? Sem ela o editor usa outra em silêncio, e o Word
 * usa uma terceira. Compara a largura de um texto com a fonte pedida na
 * frente de cada família genérica: se nenhuma largura muda, a fonte não existe.
 */
export function fonteInstalada(nome: string): boolean {
  const contexto = document.createElement("canvas").getContext("2d");
  // Sem canvas não há como saber; melhor não acusar a fonte à toa.
  if (!contexto) return true;
  const largura = (familia: string) => {
    contexto.font = `72px ${familia}`;
    return contexto.measureText(AMOSTRA).width;
  };
  return GENERICAS.some((generica) => largura(`"${nome}", ${generica}`) !== largura(generica));
}

/**
 * A altura da linha simples da fonte, em múltiplos do tamanho. O Word mede
 * a entrelinha a partir dela; o CSS do livro usa o resultado em
 * `--md-novel-linha-simples` (ADR 0002).
 */
export function linhaSimples(fonte: string): number {
  const amostra = document.createElement("span");
  amostra.textContent = "Ágp";
  amostra.style.cssText = "position:absolute;visibility:hidden;white-space:nowrap;font-size:100px;line-height:normal";
  amostra.style.fontFamily = `"${fonte}"`;
  document.body.append(amostra);
  const proporcao = amostra.offsetHeight / 100;
  amostra.remove();
  // Fora do razoável (fonte quebrada, medida zerada): fica o padrão do CSS.
  return proporcao >= 0.9 && proporcao <= 2 ? proporcao : 1.2;
}
