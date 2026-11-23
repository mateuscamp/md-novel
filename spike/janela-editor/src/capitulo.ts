// Capítulo sintético de prosa em português: parágrafos de tamanho variado,
// sem marcação, para medir só o custo do texto justificado.

const PALAVRAS = (
  "a casa ficava no alto do morro e de lá se via o rio inteiro quando a " +
  "neblina deixava ela abria as janelas antes do café e esperava o barulho " +
  "dos barcos que desciam carregados de sal e de farinha o avô dizia que a " +
  "pele guarda o que a memória solta e ninguém entendia se era conselho ou " +
  "aviso porque ele falava olhando para longe com as mãos paradas no colo"
).split(" ");

/** Gera um capítulo com `total` palavras, determinístico. */
export function capitulo(total: number): string {
  const paragrafos: string[] = [];
  let usadas = 0;
  let semente = 7;
  const proximo = () => {
    semente = (semente * 1103515245 + 12345) % 2147483648;
    return semente;
  };
  while (usadas < total) {
    const tamanho = 40 + ((proximo() >>> 12) % 120);
    const palavras: string[] = [];
    for (let i = 0; i < tamanho && usadas < total; i += 1, usadas += 1) {
      // Bits altos: os baixos deste gerador repetem em ciclos curtos.
      palavras.push(PALAVRAS[(proximo() >>> 12) % PALAVRAS.length]);
    }
    const frase = palavras.join(" ");
    paragrafos.push(frase.charAt(0).toUpperCase() + frase.slice(1) + ".");
  }
  // Uma linha por parágrafo, sem linha em branco entre eles: assim cada
  // .cm-line é um parágrafo inteiro, como no editor que vai existir.
  return paragrafos.join("\n");
}
