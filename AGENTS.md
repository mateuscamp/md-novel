# md Novel

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
- Teste funcional do app (fora do CI):
  `python3 -I -B app/teste_funcional.py target/debug/md-novel-app [captura.png]`.
  São doze passadas: salvar sozinho (com a lateral, a contagem e o realce),
  fechar a janela, exportar, tema escuro, modo foco, a escolha de tema
  guardada, criar o estilo.yaml pelo botão, o conforto (zoom, lateral,
  máquina de escrever), a escrita (aspas, travessão, busca), a sessão
  (reabrir onde parou, meta do dia), o capítulo novo e o corretor
  ortográfico, que precisa do `hunspell-pt-br` instalado. `--passada=N` roda
  uma só. Ele abre o app num
  display virtual invisível (`xvfb-run`), com a pasta de dados do app num
  temporário. Com o png ele grava também `-escuro` e `-foco`: é assim que se
  olha o visual. Rode antes de dizer que uma mudança na interface funciona.
- Nunca abra janela na tela do autor sem ele pedir: nada de `--na-tela`, de
  rodar o binário do app direto nem de `cargo tauri dev` por conta própria
  (2026-11-23: as janelas do teste atrapalharam o trabalho dele).
- Menu de aplicativos (só Linux): `make instalar` compila sem depuração e
  copia o programa, o ícone e a entrada do menu para `~/.local`;
  `make desinstalar` apaga esses três. Quem faz é `app/linux/instalar.sh`,
  testado em `app/src-tauri/tests/instalar.rs` num prefixo temporário. Não
  abre janela. Num teste, nunca no prefixo de verdade.
- Linha de comando: `cargo run -p md-novel -- estilo estilo.yaml --css editor.css --docx reference.docx`
  e `cargo run -p md-novel -- exportar --estilo estilo.yaml --saida livro.docx cap1.md cap2.md`.

## Estrutura e fronteiras

| Onde | Papel | Regra |
|---|---|---|
| `crates/estilo` | núcleo: estilo.yaml → CSS + styles.xml | Rust puro. Sem processo, sem rede, sem outro crate do projeto. Dependência nova exige mudar o ADR 0002. |
| `crates/workspace` | a pasta do livro: listar e ler capítulos, contar palavras e achar o título, e toda gravação em disco | Rust puro, sem tauri e sem processo. Único lugar que escreve arquivo. |
| `crates/exportar` | único lugar que chama o Pandoc | Só de .md para .docx (ADR 0001). Sem shell, sem entrada padrão, capítulos depois de `--`. Aviso do Pandoc volta para quem chamou, nunca é descartado. Grava pelo `workspace`. |
| `crates/cli` | adaptador fino | Lê argumentos, chama os crates. Sem regra de estilo. |
| `app/src-tauri` | adaptador fino da janela | Cada comando repassa para `Sessao` (`sessao.rs`) ou para `exportacao.rs`, os dois sem tauri e testados sem janela. Operação demorada (o Pandoc) roda com a sessão destravada. A página nunca manda caminho: a pasta vem do seletor do sistema ou da linha de comando, e capítulo só pelo nome da lista. A exceção é o nome de um capítulo novo, que o autor digita e o `workspace` confere (`Livro::criar_capitulo`). |
| `app/linux` | o instalador do menu de aplicativos (`instalar.sh`, shell POSIX) | Só o `make instalar` e o `make desinstalar` chamam. Escreve três arquivos dentro do `PREFIXO` e mais nenhum, cada um por temporário + rename, os três preparados antes de publicar o primeiro. Nunca toca a pasta do livro. É a única gravação fora do `workspace`. |
| `app/src` | a interface (TypeScript, CodeMirror 6) | Recebe o CSS pronto do Rust e não define tipografia do texto. O que o `estilo.yaml` ainda não cobre (títulos, marcadores `:::`) fica em `app.css`, com comentário dizendo isso. Regra testável vai em função pura com teste (`*.test.ts`). O tema e a moldura (`app.css`) não mudam a tipografia do texto. Dependência nova entra na lista de `fronteira.test.ts`, com o motivo no ADR 0003. |

## Regras

- O projeto tem dois nomes. `md-novel`, sem espaço e sem maiúscula, é o de
  pasta, comando, binário, identificador, classe de CSS e chave. "md Novel"
  é o que uma pessoa lê: janela, menu, mensagem, README, documentação.
  `app/src/nome.test.ts` confere a janela, a página e o README.
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
  `.docx` ou `.css`. O `estilo.yaml` só é gravado por
  `workspace::Livro::criar_estilo`: a pedido do autor (o botão do aviso), a
  partir do `exemplos/estilo.yaml` embutido, e nunca por cima de um que já
  existe, nem de um link. Capítulo (`.md`) só nasce por
  `workspace::Livro::criar_capitulo`, vazio e nunca por cima de outro, e só é gravado por
  `workspace::Capitulo::salvar`, que recusa se o arquivo mudou no disco desde
  a leitura e não escreve se o texto é o mesmo. Comando com mais de uma
  saída prepara todas antes de publicar a primeira. A exceção é o
  instalador (`app/linux/instalar.sh`): ele não é o app, grava só no
  `PREFIXO` e segue a mesma disciplina por conta própria.
- Abrir e salvar sem editar devolve os mesmos bytes: o editor trabalha em LF
  sem BOM, e o `workspace` devolve o arquivo no formato em que estava. Arquivo
  que não daria para devolver igual (finais de linha misturados, fora do
  UTF-8) é recusado na leitura, nunca "consertado".
- O app não cria arquivo na pasta do livro além do que o autor pediu para
  salvar ou criar (hoje: os capítulos e o `estilo.yaml` do botão). O que o
  app lembra (último livro, onde parou, palavras de hoje, tema, zoom) fica
  na pasta de dados dele: o último livro em `memoria.rs`, pela
  `SaidaPendente`; o resto no armazenamento da janela. Cache e configuração do app, quando existirem, vão fora dela.
- Limite ou guarda nova (tamanho, faixa, extensão) vem com o teste que
  reprova quando ela é removida.
- Contagem de palavras e título do capítulo existem duas vezes, em
  `crates/workspace/src/texto.rs` e em `app/src/texto.ts`. Mudou a regra:
  primeiro o caso novo em `crates/workspace/tests/fixtures/`, depois os dois
  lados, no mesmo commit.
- O que é título quem decide é o Pandoc, não o CommonMark. Antes de escrever
  um caso em `titulos.txt`, confira com `pandoc -f markdown -t native`;
  `crates/exportar/tests/titulos_pandoc.rs` reprova o caso que o Pandoc não
  confirma.
- Tecla ou rolagem que faz algo na janela entra em função pura com teste
  (`foco.ts`, `conforto.ts`), usa Ctrl (Cmd no Mac) e nunca vale no meio da
  composição de um acento. A tela cheia é comando do Rust, não capacidade
  nova. O zoom é só do texto: toda medida do texto, no CSS gerado e no
  `app.css`, é multiplicada por `--md-novel-zoom`; medida nova entra com ela.
- O app só troca o que o autor acabou de digitar (aspas, travessão), nunca
  texto colado nem o que já está no capítulo, e nunca onde o Markdown
  precisa do sinal reto (linha `:::`, código, link, tabela). Caso novo entra
  primeiro em `tipografia.test.ts`.
- Texto do CodeMirror que aparece para o autor é traduzido em `busca.ts`.
- O editor mostra o Markdown, não esconde: sinal digitado (`*`, `#`, `:::`)
  continua na tela. O modo foco nunca esconde os avisos.
- Caminho de imagem num capítulo vale a partir da pasta daquele capítulo
  (`+rebase_relative_paths`), não da pasta de onde o comando roda.
- O único arquivo que o md Novel apaga sozinho é temporário dele mesmo:
  `.md-novel-gravando-*` terminado em `.css`, `.docx`, `.md` ou `.yaml`, arquivo comum,
  parado há mais de uma hora. Não alargue esse padrão.
- Segurança da janela: o CSP só afrouxa `style-src` (o CodeMirror injeta CSS);
  script continua só do próprio app. Capacidade nova no
  `capabilities/default.json` ou plugin novo do Tauri exige ADR.
- Pandoc só converte de .md para .docx. Importar .docx exige um ADR que
  substitua o 0001. Num teste, o Pandoc também pode ler Markdown para
  conferir uma regra nossa contra a dele (`titulos_pandoc.rs`); .docx ele
  continua sem ler.
- Mensagem de erro em português, com o valor recebido e o formato esperado.
- Não apague comentário que explica o porquê.
