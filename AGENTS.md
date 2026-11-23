# md-novel

Estilo único para um livro em Markdown: um `estilo.yaml` gera o CSS do editor
e o reference.docx do Pandoc, e os capítulos .md soltos viram .docx.
Hoje é linha de comando; o editor ainda não existe. ADRs em `docs/adr/`.

## Comandos

- Portão único: `make check` (fmt, clippy com `-D warnings`, todos os testes).
  Precisa do `pandoc` no PATH. Verde antes de dizer pronto.
- Usar: `cargo run -- estilo estilo.yaml --css editor.css --docx reference.docx`
  e `cargo run -- exportar --estilo estilo.yaml --saida livro.docx cap1.md cap2.md`.

## Estrutura e fronteiras

| Crate | Papel | Regra |
|---|---|---|
| `crates/estilo` | núcleo: estilo.yaml → CSS + styles.xml | Rust puro. Sem processo, sem rede, sem outro crate do projeto. Dependência nova exige mudar o ADR 0002. |
| `crates/exportar` | único lugar que chama o Pandoc e que publica arquivo gerado | Só de .md para .docx (ADR 0001). Sem shell, sem entrada padrão, capítulos depois de `--`. Aviso do Pandoc volta para quem chamou, nunca é descartado. |
| `crates/cli` | adaptador fino | Lê argumentos, chama os crates, grava. Sem regra de estilo. |

## Regras

- Decisão que muda estrutura, dependência, interface entre partes ou dono do
  dado passa pela skill /arquitetura-ai-first: ADR e fitness function antes de
  fechar. Decisão já tomada pelo autor não é reaberta sem evidência nova.
- Todo código escrito, inclusive script descartável, passa por /code-review e,
  em seguida, por /code-audit sobre o mesmo alvo, partindo dos achados da review.
- TDD: escreva o teste, rode, veja falhar na asserção, só então o código. Cole
  o vermelho no relatório. Checagem de arquitetura nova precisa ser vista
  reprovar com o código quebrado de propósito.
- Propriedade nova de estilo = campo em `EstiloParagrafo` e `Parcial` +
  tradução em `css.rs` e `docx.rs` + linha na tabela de
  `toda_chave_do_schema_sai_no_css_e_no_docx` + goldens, no mesmo commit. Sem
  equivalente no Word ou no CSS: rejeitar, nunca ignorar (ADR 0002).
- Não desestruture `EstiloParagrafo` nem `Parcial` com `..`, e não use `_ =>`
  em enum do projeto: é o que faz propriedade nova quebrar a compilação.
- O manuscrito é o ativo. Nenhum comando grava em arquivo que não termine em
  `.docx` ou `.css`, e toda gravação passa por `exportar::SaidaPendente`
  (temporário na pasta de destino + rename). Comando com mais de uma saída
  prepara todas antes de publicar a primeira.
- Limite ou guarda nova (tamanho, faixa, extensão) vem com o teste que
  reprova quando ela é removida.
- Caminho de imagem num capítulo vale a partir da pasta daquele capítulo
  (`+rebase_relative_paths`), não da pasta de onde o comando roda.
- O único arquivo que o md-novel apaga sozinho é temporário dele mesmo:
  `.md-novel-gravando-*.css` ou `.docx`, arquivo comum, parado há mais de uma
  hora. Não alargue esse padrão.
- Pandoc só converte de .md para .docx. Importar .docx exige um ADR que
  substitua o 0001.
- Mensagem de erro em português, com o valor recebido e o formato esperado.
- Não apague comentário que explica o porquê.
