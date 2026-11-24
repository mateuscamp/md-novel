// Vários livros (ADR 0003): o que o app lembra de cada livro no armazenamento
// da janela vai junto quando a pasta muda de nome. Quem renomeia é o Rust.

/** Os registros por livro: onde o autor parou e a conta de hoje. A chave é
 *  o prefixo mais o caminho da pasta, como em `main.ts`. */
const PREFIXOS = ["md-novel.onde:", "md-novel.hoje:"];

/** Os pares (chave antiga, chave nova) do livro que mudou de pasta. Só a
 *  chave exata daquele caminho: `/textos/livro 2` não é `/textos/livro`. */
export function chavesParaMudar(chaves: string[], antes: string, depois: string): [string, string][] {
  if (antes === depois) return [];
  return PREFIXOS.filter((prefixo) => chaves.includes(prefixo + antes)).map((prefixo) => [
    prefixo + antes,
    prefixo + depois,
  ]);
}
