// O painel de buscar e substituir é o do CodeMirror (`@codemirror/search`).
// Ele vem em inglês; aqui fica a tradução de cada texto dele. O teste confere
// contra o pacote instalado que nenhum ficou de fora.

export const FRASES_DA_BUSCA: Record<string, string> = {
  Find: "Buscar",
  Replace: "Substituir por",
  next: "próximo",
  previous: "anterior",
  all: "todos",
  "match case": "maiúsculas e minúsculas",
  regexp: "expressão regular",
  "by word": "palavra inteira",
  replace: "substituir",
  "replace all": "substituir todos",
  close: "fechar",
  "current match": "ocorrência atual",
  "on line": "na linha",
  "replaced $ matches": "$ ocorrências substituídas",
  "replaced match on line $": "ocorrência substituída na linha $",
  "Go to line": "Ir para a linha",
  go: "ir",
};
