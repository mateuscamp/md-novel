// O capítulo novo: a sugestão de nome que aparece no campo. Quem confere o
// nome e cria o arquivo é o Rust (`workspace::Livro::criar_capitulo`).

/** O começo do nome do próximo capítulo, continuando a numeração do último:
 *  `02-a-casa.md` sugere `03-`. Sem número no último capítulo não há o que
 *  continuar; num livro vazio a sugestão é `01-`. */
export function sugestaoDeNome(nomes: string[]): string {
  const ultimo = nomes.at(-1);
  if (ultimo === undefined) return "01-";
  // Até nove algarismos: mais do que isso não é numeração de capítulo.
  const achado = /^(\d{1,9})([- ]?)/.exec(ultimo);
  if (achado === null) return "";
  const [, numero = "", separador = ""] = achado;
  return String(Number(numero) + 1).padStart(numero.length, "0") + separador;
}

/** O nome do capítulo como aparece no campo de renomear: sem o `.md`, que
 *  o Rust põe de volta. Só `.md` sozinho não é extensão: fica como está. */
export function nomeParaEditar(nome: string): string {
  return nome.length > 3 && nome.toLowerCase().endsWith(".md") ? nome.slice(0, -3) : nome;
}

/** A tecla que abre o campo de renomear no capítulo da lateral: F2, como
 *  no explorador de arquivos, e nunca no meio da composição de um acento. */
export const TECLA_DE_RENOMEAR = "F2";
export function teclaDeRenomear(tecla: string, compondo: boolean): boolean {
  return !compondo && tecla === TECLA_DE_RENOMEAR;
}
