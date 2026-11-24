# 0003 — O programa de desktop é Tauri 2 com CodeMirror 6

**Status:** Aceito
**Data:** 2026-11-23
**Driver ★:** Fidelidade de estilo editor ↔ .docx

## Contexto

O md Novel já compila o `estilo.yaml` em CSS e reference.docx e exporta pela
linha de comando (ADR 0001 e 0002). O autor pediu um programa de desktop: abrir
a pasta do livro (inclusive no Dropbox), editar os capítulos vendo a
tipografia do livro e exportar, no Linux (Hyprland, RTX 4070) e no MacBook M1.
O autor escolheu Rust antes desta análise; essa escolha é um driver, não uma
conclusão.

Quantum atual: uma ferramenta de linha de comando (três crates). Previsto: um
app desktop, um quantum, com os dados (*.md, livro.yaml, estilo.yaml) na pasta
do autor e cache fora dela.

## Drivers

| ★ | Característica | Origem | Como se mede |
|---|---|---|---|
| ★ | Fidelidade de estilo editor ↔ .docx | "manter o .md com a estrutura que o docx tem" | o CSS do `crates/estilo` aplicado sem tradução à linha do editor; teste de subconjunto do ADR 0002 |
| | Digitação fluida num capítulo longo | romance: capítulos de 5 a 15 mil palavras | latência tecla→quadro p95 ≤ 33 ms num capítulo de 10 mil palavras justificado |
| | Integridade do texto | o manuscrito é o ativo | abrir e salvar sem editar devolve os mesmos bytes |
| | Workspace portátil e sync seguro | "aberto de dentro de uma pasta do Dropbox" | só arquivos do autor na pasta; mudança externa nunca sobrescrita |
| | Portabilidade | Arch/Hyprland/Nvidia + M1 | build e spike aprovados nas duas máquinas |
| | Rust no núcleo | escolha do autor | núcleo e adaptador em Rust; `crates/estilo` e `crates/exportar` reaproveitados sem ponte |

## Alternativas consideradas

| Opção | Ganha | Perde | Por que não |
|---|---|---|---|
| Electron + CodeMirror 6 | o mesmo Chromium nas duas máquinas (elimina o risco do WebKitGTK); um conjunto de ferramentas só (Node/TS) | app de ~150 MB; o núcleo em Rust vira binário externo ou módulo nativo | plano B: entra se o spike reprovar |
| PySide6 (como o novelWriter) | um runtime só | editor com live preview e justificação caro de fazer bem; joga fora o Rust | contraria o driver do autor |
| GUI Rust nativa (egui, iced) | sem webview | edição de prosa rica imatura; o CSS do ADR 0002 não serve | contraria o driver ★ |
| **Tauri 2 + CodeMirror 6** | Rust no núcleo, binário pequeno, o CSS do `crates/estilo` vale direto no editor | WebKitGTK no Linux é o elo fraco (Wayland + Nvidia); dois conjuntos de ferramentas (Rust e Node/TS) | — |

## Decisão

Tauri 2 com CodeMirror 6. O critério abaixo foi fixado antes de escrever o
spike, para as duas máquinas. O spike mora na branch `spike/0003-janela-editor` e não é mergeado.

**O que o spike precisa responder:**

1. O WebKitGTK desenha a janela no Hyprland com a RTX 4070 sem variável de
   ambiente que desligue a aceleração (`WEBKIT_DISABLE_DMABUF_RENDERER`,
   `WEBKIT_DISABLE_COMPOSITING_MODE`)?
2. Num capítulo de 10 mil palavras com o CSS do `exemplos/estilo.yaml`
   (justificado, entrelinha 1,5, recuo de 1,25 cm), a latência entre uma
   inserção no meio do texto e o quadro seguinte tem p95 ≤ 33 ms?
3. Rolando do início ao fim, o intervalo entre quadros tem p95 ≤ 33 ms?
4. Na tela, a justificação, o recuo e a entrelinha saem como o CSS manda
   (captura de tela)?
5. As perguntas 1 a 4 de novo no M1 (WKWebView), rodadas pelo autor.

**Inverte para Electron (ADR 0004 substitui este)** se qualquer resposta for
não numa das duas máquinas e não houver contorno que mantenha a aceleração.

## Resultado do spike no Linux (2026-11-23)

Máquina do autor: Hyprland (Wayland nativo, sem XWayland), RTX 4070,
webkit2gtk-4.1 2.54.3, Tauri 2.12.7 (a do Cargo.lock do spike), CodeMirror 6
(view 6.43.15). Capítulo
sintético de 10 mil palavras (98 parágrafos, 49 mil caracteres), CSS gerado
do `exemplos/estilo.yaml`. Código em `spike/janela-editor`, captura em
`spike/janela-editor/evidencias/linux-hyprland-rtx4070.png`.

| Pergunta | Resultado | Leitura |
|---|---|---|
| 1. Desenha sem desligar a aceleração? | sim: nenhuma variável `WEBKIT_DISABLE_*` definida; janela Wayland nativa | passa |
| 2. Inserção → quadro, p95 ≤ 33 ms? | p50 32 ms, **p95 34 ms**, máx 35 ms | **não passa pela letra do critério, por 1 ms** (ver abaixo) |
| 3. Rolagem, p95 ≤ 33 ms entre quadros? | p50 16 ms, p95 18 ms, máx 19 ms (173 quadros) | passa |
| 4. Tipografia como o CSS manda? | justificado, recuo de 47,2 px (1,25 cm) e linha de 28,8 px (12 pt × 1,2 × 1,5), conferidos na captura e no estilo computado | passa, com a ressalva da fonte |
| 5. As mesmas no M1 | não rodado | pendente, com o autor |

**Sobre a pergunta 2.** O método mede até o segundo quadro depois da
inserção, e o WebKitGTK entrega quadros a 60 Hz: o piso do método é de 33,3
ms, acima do limite que eu tinha fixado. O critério, como escrito, não
separa "rápido" de "lento" nesta tela; foi erro de quem o escreveu, e ele
não foi reescrito depois de ver o número. As medidas que o spike passou a
tirar junto dizem o seguinte: o trabalho da tecla (aplicar a mudança e
refazer o layout) custa p95 de 2 a 3 ms e no máximo 4 ms nas quatro
rodadas, e o intervalo entre quadros durante as 200 inserções fica em p95
17 ms, máx 19 ms, isto é, nenhum quadro perdido. A decisão de aceitar essa leitura é do autor.

**Ainda sem medir:** tecla de verdade (o spike mede quando alguém digita na
janela; as inserções acima são sintéticas) e o M1.

**O que o spike ensinou para a reconstrução (de-spike):**

- O Tauri acrescenta nonce ao `style-src`, e com nonce o navegador ignora
  `'unsafe-inline'`. O CSS que o CodeMirror injeta em tempo de execução é
  bloqueado e o editor fica sem o tema base (sem rolagem própria, desenhando
  o texto inteiro). No spike isso foi contornado desligando a modificação do
  `style-src`; no app, usar `EditorView.cspNonce` com o nonce do Tauri.
- O editor precisa de altura fixa e `.cm-scroller { overflow: auto }`; sem
  isso o CodeMirror não virtualiza.
- O WebKitGTK limita os quadros a 60 Hz mesmo num monitor de 165 Hz.
- A fonte do exemplo (EB Garamond) não está instalada nesta máquina; a tela
  usou a fonte de reserva. O app precisa avisar quando a fonte do estilo não
  existe, ou tela e Word usam fontes diferentes sem ninguém notar.
- O spike não é a base do app: ordem dos testes na reconstrução é abrir
  pasta e listar capítulos, abrir e salvar sem mudar um byte, aplicar o CSS
  do estilo, exportar.

## A primeira fatia do app (2026-11-23)

Abre a pasta, lista os capítulos, edita com o CSS do livro (inclusive os
estilos nomeados, por um atributo em cada linha do bloco) e salva. Decisões
tomadas nela:

- **A página não manda caminho.** A pasta vem do seletor do sistema, aberto
  pelo Rust, ou da linha de comando; capítulo só pelo nome que está na lista.
  A janela não recebe permissão de diálogo nem de arquivos.
- **Salvar sozinho** 1,5 s depois da última tecla, ao trocar de capítulo e ao
  sair da janela. É seguro porque salvar confere antes se o arquivo mudou por
  fora; nesse caso para e pede para recarregar.
- **CSP:** `style-src` aceita estilo inline, e a troca automática de nonce do
  Tauri fica desligada só para estilo. O plano do spike era passar o nonce ao
  CodeMirror (`EditorView.cspNonce`); ficou para depois, porque o app só
  mostra o capítulo como texto e script continua restrito ao próprio app.
- **Janela de corrida ao salvar:** entre conferir o disco e trocar o arquivo
  cabe uma mudança externa de milissegundos. Aceito; fechar exigiria trava de
  arquivo, que o Dropbox não respeita.
- **Fica de fora desta fatia:** `livro.yaml` com a ordem dos capítulos (vale
  a ordem de nome), esconder a linha em branco entre parágrafos, tipografia
  de título, vigiar a pasta enquanto o app está aberto, e o instalador.

## A segunda fatia: exportar pelo app (2026-11-23)

O botão "Exportar .docx…" gera o livro inteiro. Decisões:

- **O destino vem do "salvar como" do sistema**, aberto pelo Rust, como a
  pasta: a página continua sem mandar caminho e sem permissão nova.
- **Salva antes de exportar.** A conversão lê os capítulos do disco; se
  sobrou texto que não pôde ser salvo, a exportação não começa.
- **A sessão não fica travada durante o Pandoc.** O que exportar (capítulos
  em ordem e o texto do estilo) é tirado da sessão de uma vez; salvar um
  capítulo continua possível enquanto a conversão roda.
- **Sem `estilo.yaml` exporta com o padrão do Pandoc e avisa; com ele
  inválido não exporta.** Um .docx com a cara errada passaria por certo.
- **No macOS, aberto pelo Finder, o app pode não achar o Pandoc** (o PATH de
  programa gráfico não inclui o Homebrew). Fica para quando houver
  instalador; aberto pelo terminal, funciona.

## A terceira fatia: a moldura (2026-11-23)

O autor pediu o app "mais refinado, como o novelWriter e o Zettlr", e
escolheu as quatro partes: lateral com título e palavras, barra de status,
Markdown à vista e modo foco, com tema claro e escuro. Decisões:

- **A contagem de palavras e o título do capítulo são regra do `workspace`**
  (`texto.rs`), que é quem lê os capítulos para montar a lista. A interface
  repete as duas funções (`app/src/texto.ts`) para contar enquanto o autor
  digita, sem ir ao Rust a cada tecla. Duas implementações da mesma regra
  divergem com o tempo; o que as segura é um arquivo de casos só
  (`crates/workspace/tests/fixtures/`), lido pelos testes dos dois lados.
  Perde-se: toda mudança na regra é feita duas vezes.
- **A lista do livro passou a ser `{nome, titulo, palavras}`** no lugar de só
  os nomes. Abrir a pasta lê todos os capítulos uma vez. Capítulo que não dá
  para ler continua na lista, sem números; o motivo aparece ao abri-lo.
- **Markdown à vista, nunca escondido.** Itálico, negrito e títulos aparecem
  como vão sair, e os sinais digitados (`*`, `#`, `>`) ficam na tela,
  apagados. Esconder o sinal fora do cursor, como o Zettlr faz, muda o que o
  autor vê do que está no arquivo; aqui o arquivo é o que está na tela.
- **Só o analisador `@lezer/markdown`, sem `@codemirror/lang-markdown`.** O
  pacote completo traz junto as linguagens HTML, CSS e JavaScript, que um
  romance não usa. O analisador lê CommonMark; o `:::` do Pandoc ele vê como
  texto, e quem cuida dele continua sendo o `marcarLinhas`. Perde-se: o que
  o Pandoc lê diferente do CommonMark pode aparecer diferente na tela.
- **Título é o que o Pandoc lê como título, não o CommonMark.** A primeira
  versão desta fatia seguia o CommonMark e a revisão achou a diferença: para
  o Pandoc o título não pode vir colado num parágrafo nem recuado, pode ter
  mais de seis `#` e perde os `#` do fim mesmo colados. A regra da lateral
  (`titulo_do_texto`) e a da linha do editor (`marcarLinhas`) passaram a ser
  essa, e o tamanho de título só vale em linha que o `marcarLinhas` aceita.
  Fica de fora o título sublinhado (`===`, `---`), que o Pandoc lê e a tela
  ainda não mostra.
- **O tema é da moldura, não do livro.** A escolha (sistema, claro ou
  escuro) fica no `localStorage` da janela, que o sistema guarda na pasta de
  dados do app, fora da pasta do livro. O CSS do `estilo.yaml` não define
  cor, então o tema não muda o que vai para o .docx.
- **No modo foco os avisos continuam na tela.** Somem a lateral e as barras;
  texto que não foi salvo não pode passar despercebido.
- **O teste funcional roda com a pasta de dados do app num temporário** e
  recebe o tema pelo roteiro de teste, sem guardar: ele não lê nem muda a
  escolha de quem usa o programa na mesma máquina.

## Consequências

**Ganha:** o núcleo em Rust que já existe entra no app sem ponte; o CSS gerado
vale no editor sem tradução.

**Perde:**
- Dois conjuntos de ferramentas para sempre (cargo e npm), com dois portões.
- O WebKitGTK e o WKWebView diferem entre si: a mesma tela pode sair
  diferente no Linux e no Mac.
- O CI do Linux passa a precisar do `webkit2gtk-4.1` para compilar o app.

**Aceito conscientemente:** a discordância registrada na análise do debate:
o Electron é um conjunto de ferramentas só e elimina o maior risco técnico.
Ele não foi escolhido porque o autor escolheu Rust; se o spike reprovar, ele
entra sem nova discussão.

## Identificador do app (2026-11-23)

`io.github.mateuscamp.md-novel`, no lugar de `dev.mdnovel.app`. O Tauri
avisava que um identificador terminado em `.app` conflita com a extensão de
pacote do macOS, e `dev.mdnovel` sugeria um domínio que o autor não tem. A
troca foi feita antes de existir instalador porque o identificador vira o
nome das pastas de dados do app e a identidade dele no macOS: depois disso,
trocar deixa para trás o que estiver guardado no nome antigo.

## O nome e o menu de aplicativos (2026-11-23)

O autor procurou o app no menu e não achou: só existia o binário dentro de
`target/`. E disse como o projeto se chama: "md Novel" onde uma pessoa lê;
`md-novel` é o nome de pasta e comando, sem espaço e sem maiúscula.

- **Dois nomes, cada um no seu lugar.** "md Novel" na janela
  (`productName` e título), na página, nas mensagens e na documentação.
  `md-novel` continua em repositório, crates, binários, identificador,
  classes de CSS e chaves guardadas: nada do que já está em disco muda de
  nome.
- **`make instalar`, só no Linux.** Compila sem depuração e copia três
  arquivos para `~/.local` (ou outro `PREFIXO`): `bin/md-novel-app`, o
  ícone e `share/applications/io.github.mateuscamp.md-novel.desktop`. Quem
  copia é `app/linux/instalar.sh`: prepara os três antes de publicar o
  primeiro, troca cada um por temporário e rename, e recusa prefixo que não
  cabe na linha `Exec`. `make desinstalar` apaga os três pelo caminho exato.
- **O empacotador do Tauri continua desligado** (`bundle.active: false`).
  Ligá-lo daria `.deb`, AppImage e `.app`, mas pede ícones em todos os
  tamanhos, assinatura no Mac e uma decisão de distribuição que ninguém
  pediu. O custo aceito: no Mac o app segue aberto pelo terminal, e a nota
  do Pandoc fora do PATH (segunda fatia) continua valendo.
- **A entrada do menu aponta para o caminho inteiro do programa**, e o
  ícone também: `~/.local/bin` nem sempre está no PATH de quem abre o menu.
  Mover a pasta depois de instalar quebra a entrada; `make instalar` de
  novo conserta.

## Criar o estilo.yaml pelo app (2026-11-24)

O autor abriu um livro sem `estilo.yaml` e não entendeu o aviso: ele mandava
copiar "o exemplos/estilo.yaml do md Novel", um arquivo dentro do
repositório, que quem instalou pelo menu não sabe onde está. Ele escolheu um
botão no próprio aviso.

- **O aviso da pasta sem estilo diz para que o arquivo serve e traz o botão
  "Criar estilo.yaml".** O Rust manda esse aviso à parte dos outros
  (`aviso_sem_estilo`), porque só ele tem ação.
- **O modelo é o `exemplos/estilo.yaml`, embutido no programa.** Um arquivo
  só: o exemplo que o README manda copiar e o que o botão grava não
  divergem. O app instalado não depende da pasta do projeto.
- **É a única gravação do app na pasta do livro que não é salvar um
  capítulo**, e só acontece no clique. Quem grava é
  `workspace::Livro::criar_estilo`, por temporário e troca que recusa
  sobrescrever: se já existe qualquer coisa chamada `estilo.yaml` (arquivo,
  link, link quebrado), nada é gravado. Um estilo inválido não ganha botão:
  ele já é o estilo do autor, só com um erro.
- **A página não manda caminho nem conteúdo.** O comando não tem argumento.
- **O capítulo aberto não é relido** ao criar o estilo: só o CSS muda. O que
  foi digitado continua na tela e vai para o disco pelo caminho de sempre.
- **Custo aceito:** o temporário `.md-novel-gravando-*` pode terminar também
  em `.yaml`, e a faxina de órfãos passou a recolhê-lo. E o estilo criado
  pede as fontes do modelo (EB Garamond e Courier Prime); em máquina sem
  elas o aviso do estilo dá lugar ao aviso da fonte.
- **Fica de fora:** editar o estilo por dentro do app. Mudou o arquivo,
  fecha e abre.

## A quarta fatia: conforto de quem escreve (2026-11-24)

O autor pediu que Ctrl+rolagem aproximasse o texto, um botão simples para
esconder a lateral e "elementos de qualidade de vida do Zettlr e do
novelWriter". Esta fatia traz os que são só da janela; busca, corretor,
aspas, sessão e capítulo novo vêm depois.

- **O zoom é da janela inteira**, como o de um navegador e o do Zettlr:
  Ctrl+rolagem, Ctrl+=, Ctrl+- e Ctrl+0, de 50% a 300%, de 10 em 10. Quem
  aplica é o WebView (`set_zoom`), por um comando do Rust que confere a
  faixa. Zoom só do texto foi descartado: `zoom` de CSS desalinha o clique
  e o cursor do CodeMirror, e escalar as medidas exigiria mudar o CSS que o
  crate `estilo` gera para todos (ADR 0002). O custo: a lateral e as barras
  crescem junto. Inverte se o autor quiser a moldura do mesmo tamanho.
- **A lateral se esconde com o botão ☰ ou F9.** O modo foco continua
  escondendo a lateral e as barras por conta própria.
- **Máquina de escrever (botão ou F10):** digitar ou andar com o teclado
  leva a linha do cursor para o meio da área de texto. Com o mouse não: a
  tela fugiria de debaixo do ponteiro.
- **Tela cheia em F11**, pelo Rust (`set_fullscreen`), sem botão.
- **Nenhuma capacidade nova do Tauri:** zoom e tela cheia são comandos do
  próprio app, não a API de janela exposta à página.
- **Zoom, lateral e máquina de escrever ficam guardados** no armazenamento
  da janela, como o tema, fora da pasta do livro.
- **O atalho usa Ctrl (Cmd no Mac), nunca Super nem Alt:** no Linux a tecla
  Super é do gerenciador de janelas.

## A quinta fatia: buscar, aspas e travessão (2026-11-24)

Segunda parte das facilidades pedidas pelo autor.

- **Buscar e substituir é o painel do CodeMirror** (`@codemirror/search`,
  dependência nova, a oitava da interface): Ctrl+F e Ctrl+H abrem o mesmo
  painel, no alto da folha. Escrever uma busca própria seria refazer o que o
  editor já resolve (realce das ocorrências, expressão regular, substituir
  dentro do desfazer). O painel vem em inglês; `busca.ts` traduz cada
  texto, e um teste confere a tradução contra o pacote instalado.
- **Aspas curvas e travessão ao digitar**, como no novelWriter: `"` vira
  “ ou ”, `'` vira ‘ ou ’, e `--` vira —. Ligado de fábrica, com um botão na
  barra de baixo para desligar; a escolha fica guardada. O arquivo passa a
  guardar os caracteres tipográficos, e o Pandoc os leva ao .docx como são.
- **Onde o Markdown precisa do sinal reto, nada é trocado:** na linha de um
  bloco `:::` (o `custom-style="..."`), em código, em endereço de link e em
  linha de tabela. O terceiro hífen numa linha só de travessão devolve
  `---`, a quebra de cena.
- **Só a tecla digitada é trocada.** Texto colado entra como veio, e o que
  já está no capítulo não é reescrito.

## A sexta fatia: a sessão de escrita (2026-11-24)

Terceira parte das facilidades pedidas pelo autor.

- **Aberto sem pasta (pelo menu), o app volta ao último livro.** Quem lembra
  é o Rust: `ultimo-livro.txt`, na pasta de dados do app, gravado pela
  `SaidaPendente` a cada livro aberto. A página continua sem mandar caminho.
  Se a pasta sumiu ou o registro está estragado, o app abre sem livro. A
  abertura inicial passou para o `setup` do Tauri, que é onde a pasta de
  dados fica conhecida.
- **Em cada livro, o app volta ao capítulo e ao cursor de onde o autor
  parou.** Isso fica no armazenamento da janela, um registro por livro, com
  o caminho que veio do Rust na chave.
- **"Hoje: N palavras"** na barra de baixo: o total do livro agora menos o
  total de quando o dia começou, por livro. Clicar define a meta do dia (uma
  só, para todos os livros); com ela, "Hoje: N de M palavras". Cortar texto
  desconta, e o número pode ficar negativo.
- **Nada disso vai para a pasta do livro.**
- **Custo aceito:** o temporário da `SaidaPendente` na pasta de dados
  termina em `.txt`, que a faxina de órfãos não recolhe; uma queda no meio
  da gravação deixa lá um arquivo de bytes. E capítulos que chegam por fora
  (a sincronização) contam como escritos hoje.

## Capítulo novo pelo app (2026-11-24)

O autor pediu para criar capítulo sem sair do programa.

- **"Novo capítulo…" na lateral abre um campo com a sugestão do próximo
  número** (`02-a-casa.md` sugere `03-`); Enter cria um arquivo `.md` vazio
  na pasta do livro e abre nele. O capítulo aberto é salvo antes.
- **É a primeira vez que a página manda um nome que não veio da lista do
  livro.** Só o nome, nunca um caminho, e quem confere é o `workspace`:
  sem barra, sem `..`, sem começar com ponto, sem os caracteres que o
  Windows de quem sincroniza a pasta recusa, até 100 caracteres. O `.md`
  entra sozinho.
- **O que já existe nunca é trocado**, pela mesma troca que recusa
  sobrescrever usada no `estilo.yaml`.
- **Nasce vazio.** O app não escreve título nem modelo: o que entra no
  manuscrito é do autor.
- **Fica de fora:** renomear, apagar e reordenar capítulos. Continuam sendo
  feitos na pasta, e a ordem continua sendo a do nome.

## Governança

| O que verifica | Mecanismo | Onde roda | Falha quando |
|---|---|---|---|
| Critério do spike | medição embutida no spike (latência e quadros) e captura de tela | Hyprland/RTX 4070 e M1, uma vez | p95 > 33 ms, janela sem aceleração ou tipografia errada |
| Núcleo sem Tauri | `crates/estilo/tests/fronteira.rs` (já existe) e o mesmo teste em `crates/exportar` | `make check` | `tauri` aparece como dependência de um crate do núcleo |
| Contagem e título iguais no Rust e na interface | `crates/workspace/tests/texto.rs` e `app/src/texto.test.ts`, os dois lendo `crates/workspace/tests/fixtures/` | `make check` | um dos lados dá outro número ou outro título para um caso do arquivo |
| Os casos de título são o que o Pandoc lê | `crates/exportar/tests/titulos_pandoc.rs`, que passa cada caso de `titulos.txt` pelo Pandoc | `make check` | o arquivo de casos afirma um título que o Pandoc não dá |
| Aspas e travessão | `app/src/tipografia.test.ts` | `make check` | uma aspa abre onde devia fechar, dois hifens não viram travessão, ou algo é trocado numa linha `:::`, em código em linha ou em tabela |
| A busca fala português | `app/src/busca.test.ts`, que lê os textos do pacote instalado | `make check` | o `@codemirror/search` tem um texto sem tradução, ou a tradução perde o `$` do número |
| Escrita na janela de verdade | `app/teste_funcional.py`, passada 9 | à mão, fora do CI | as teclas não viram aspas curvas e travessão, o botão não desliga a troca, Ctrl+F não abre o painel em português ou a busca não seleciona a palavra |
| O último livro | testes de `app/src-tauri/src/memoria.rs` | `make check` | um registro estragado, relativo, grande demais ou de pasta que sumiu abre alguma coisa, ou sobra temporário na pasta de dados |
| Onde parou e a conta de hoje | `app/src/onde.test.ts` e `app/src/meta.test.ts` | `make check` | um capítulo que sumiu é reaberto, o dia vira e a conta não recomeça, ou uma meta fora da faixa é aceita |
| A sessão na janela de verdade | `app/teste_funcional.py`, passada 10 | à mão, fora do CI | aberto sem pasta, o app não volta ao último livro, ao capítulo e ao cursor, a conta de hoje ou a meta se perdem, ou aparece arquivo na pasta do livro |
| Capítulo novo não sai da pasta nem sobrescreve | `crates/workspace/tests/livro.rs` (`criar_capitulo_*`, `nome_de_capitulo_*`) | `make check` | um nome com barra, `..`, ponto no começo ou caractere reservado é aceito, um capítulo que existe é trocado, ou aparece arquivo fora da pasta do livro |
| Capítulo novo na janela de verdade | `app/teste_funcional.py`, passada 11 | à mão, fora do CI | o botão não sugere o próximo número, o capítulo não é criado vazio, listado e aberto, o texto do capítulo anterior não é salvo antes, ou o nome repetido é aceito |
| Dependências da interface | `app/src/fronteira.test.ts` | `make check` | o `package.json` do app ganha ou perde uma dependência sem a lista do teste mudar junto |
| O nome que as pessoas leem | `app/src/nome.test.ts` | `make check` | a janela, a página ou a primeira linha do README deixam de dizer "md Novel", ou o identificador ganha espaço ou maiúscula |
| Instalar no menu | `app/src-tauri/tests/instalar.rs`, que roda o `app/linux/instalar.sh` num prefixo temporário | `make check` | instalar cria algo além dos três arquivos, a entrada do menu sai diferente, um prefixo que não cabe na linha `Exec` é aceito, roda fora do Linux, ou desinstalar apaga um vizinho |
| O estilo criado pelo app nunca sobrescreve | `crates/workspace/tests/livro.rs` (`criar_estilo_*`) | `make check` | criar o estilo com um `estilo.yaml` já na pasta (arquivo ou link) muda algum byte, ou deixa outro arquivo na pasta |
| O botão grava o exemplo do projeto e devolve o livro com CSS | testes `criar_estilo_*` de `app/src-tauri/src/sessao.rs` | `make check` | o arquivo gravado difere de `exemplos/estilo.yaml`, o livro volta sem CSS ou o aviso continua |
| Criar o estilo na janela de verdade | `app/teste_funcional.py`, passada 7 | à mão, fora do CI | o aviso sem o botão, a linha do corpo sem o alinhamento do modelo depois do clique, a frase digitada perdida ou arquivo a mais na pasta |
| Regras do conforto | `app/src/conforto.test.ts` | `make check` | o zoom sai da faixa ou anda fora do passo, uma tecla de zoom vale sem Ctrl, um atalho vale no meio de um acento, ou a faixa da interface difere da do Rust |
| O Rust confere o zoom | testes de `app/src-tauri/src/janela.rs` | `make check` | um zoom fora de 50% a 300% é aceito |
| O conforto na janela de verdade | `app/teste_funcional.py`, passada 8 | à mão, fora do CI | o zoom não muda a largura útil da janela, F9 não esconde a lateral, F10 não leva o cursor para o meio, ou as três escolhas não voltam ao abrir de novo |
| A moldura na janela de verdade | `app/teste_funcional.py`, passadas 1, 4, 5 e 6 | à mão, fora do CI | contagem na tela diferente da conta do teste, título colado com tamanho de título, tema sem efeito ou esquecido ao reabrir, barras visíveis no modo foco ou aviso escondido nele |

Regra derivada no `AGENTS.md`, quando aceito: o app (`src-tauri`) é
adaptador fino; regra de estilo e chamada ao Pandoc continuam nos crates.

## Notas

Criado em 2026-11-23 · Aprovado em 2026-11-23 pelo autor, depois do spike
no Linux e antes do teste no M1: ele aceitou a leitura da pergunta 2 (nenhum
quadro perdido, 2 a 3 ms por tecla) e mandou começar o app com o M1 em
paralelo. Se o M1 reprovar, um ADR novo troca para Electron e substitui este.
O spike fica só na branch `spike/0003-janela-editor`.
