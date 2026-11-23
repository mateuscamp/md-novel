# md-novel

Estilo único para um livro em Markdown: um `estilo.yaml` gera o CSS do editor
e o reference.docx do Pandoc, e os capítulos .md soltos viram .docx.
Tem um programa de desktop (Tauri 2 + CodeMirror 6, ADR 0003) e uma linha de
comando. ADRs em `docs/adr/`.

## Comandos

- Portão único: `make check` (interface: tipos, testes e build; Rust: fmt,
  clippy com `-D warnings` e todos os testes). Precisa do `pandoc` no PATH, do
  Node e, no Linux, do `webkit2gtk-4.1`. Verde antes de dizer pronto.
- CI: `.github/workflows/check.yml` roda o mesmo `make check` no Linux e no
  Mac arm64, com Rust 1.96.0, Node 22.23.5 e Pandoc 3.11 fixos. Trocar uma
  dessas versões é trocar no workflow e na máquina de desenvolvimento juntos.
- App: `cd app && cargo tauri build --debug --no-bundle`, depois
  `target/debug/md-novel-app <pasta do livro>`. `cargo run -p md-novel-app`
  sozinho abre uma janela em branco: em depuração ele espera o `vite` no ar.
- Teste funcional do app (precisa de sessão gráfica, fora do CI):
  `python3 -I app/teste_funcional.py target/debug/md-novel-app`. Rode antes
  de dizer que uma mudança na interface funciona.
- Linha de comando: `cargo run -p md-novel -- estilo estilo.yaml --css editor.css --docx reference.docx`
  e `cargo run -p md-novel -- exportar --estilo estilo.yaml --saida livro.docx cap1.md cap2.md`.

## Estrutura e fronteiras

| Onde | Papel | Regra |
|---|---|---|
| `crates/estilo` | núcleo: estilo.yaml → CSS + styles.xml | Rust puro. Sem processo, sem rede, sem outro crate do projeto. Dependência nova exige mudar o ADR 0002. |
| `crates/workspace` | a pasta do livro: listar e ler capítulos, e toda gravação em disco | Rust puro, sem tauri e sem processo. Único lugar que escreve arquivo. |
| `crates/exportar` | único lugar que chama o Pandoc | Só de .md para .docx (ADR 0001). Sem shell, sem entrada padrão, capítulos depois de `--`. Aviso do Pandoc volta para quem chamou, nunca é descartado. Grava pelo `workspace`. |
| `crates/cli` | adaptador fino | Lê argumentos, chama os crates. Sem regra de estilo. |
| `app/src-tauri` | adaptador fino da janela | Cada comando repassa para `Sessao` (`sessao.rs`, sem tauri, testada sem janela). A página nunca manda caminho: a pasta vem do seletor do sistema ou da linha de comando, e capítulo só pelo nome da lista. |
| `app/src` | a interface (TypeScript, CodeMirror 6) | Recebe o CSS pronto do Rust e não define tipografia do texto. O que o `estilo.yaml` ainda não cobre (títulos, marcadores `:::`) fica em `app.css`, com comentário dizendo isso. Regra testável vai em função pura com teste (`*.test.ts`). |

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
- O manuscrito é o ativo. Toda gravação passa por `workspace::SaidaPendente`
  (temporário na pasta de destino + rename). Arquivo gerado só termina em
  `.docx` ou `.css`. Capítulo (`.md`) só é gravado por
  `workspace::Capitulo::salvar`, que recusa se o arquivo mudou no disco desde
  a leitura e não escreve se o texto é o mesmo. Comando com mais de uma
  saída prepara todas antes de publicar a primeira.
- Abrir e salvar sem editar devolve os mesmos bytes: o editor trabalha em LF
  sem BOM, e o `workspace` devolve o arquivo no formato em que estava. Arquivo
  que não daria para devolver igual (finais de linha misturados, fora do
  UTF-8) é recusado na leitura, nunca "consertado".
- O app não cria arquivo na pasta do livro além do que o autor pediu para
  salvar. Cache e configuração do app, quando existirem, vão fora dela.
- Limite ou guarda nova (tamanho, faixa, extensão) vem com o teste que
  reprova quando ela é removida.
- Caminho de imagem num capítulo vale a partir da pasta daquele capítulo
  (`+rebase_relative_paths`), não da pasta de onde o comando roda.
- O único arquivo que o md-novel apaga sozinho é temporário dele mesmo:
  `.md-novel-gravando-*` terminado em `.css`, `.docx` ou `.md`, arquivo comum,
  parado há mais de uma hora. Não alargue esse padrão.
- Segurança da janela: o CSP só afrouxa `style-src` (o CodeMirror injeta CSS);
  script continua só do próprio app. Capacidade nova no
  `capabilities/default.json` ou plugin novo do Tauri exige ADR.
- Pandoc só converte de .md para .docx. Importar .docx exige um ADR que
  substitua o 0001.
- Mensagem de erro em português, com o valor recebido e o formato esperado.
- Não apague comentário que explica o porquê.
