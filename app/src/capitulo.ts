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
