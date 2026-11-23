# 0003 — O programa de desktop é Tauri 2 com CodeMirror 6, se passar no spike da janela

**Status:** Proposto
**Data:** 2026-11-23
**Driver ★:** Fidelidade de estilo editor ↔ .docx

## Contexto

O md-novel já compila o `estilo.yaml` em CSS e reference.docx e exporta pela
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

Proposta: Tauri 2 com CodeMirror 6. Ela só é aceita depois do spike da janela
passar no critério abaixo, fixado antes de escrever o spike, nas duas
máquinas. O spike mora na branch `spike/0003-janela-editor` e não é mergeado.

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

## Governança

| O que verifica | Mecanismo | Onde roda | Falha quando |
|---|---|---|---|
| Critério do spike | medição embutida no spike (latência e quadros) e captura de tela | Hyprland/RTX 4070 e M1, uma vez | p95 > 33 ms, janela sem aceleração ou tipografia errada |
| Núcleo sem Tauri | `crates/estilo/tests/fronteira.rs` (já existe) e o mesmo teste em `crates/exportar` | `make check` | `tauri` aparece como dependência de um crate do núcleo |

Regra derivada no `AGENTS.md`, quando aceito: o app (`src-tauri`) é
adaptador fino; regra de estilo e chamada ao Pandoc continuam nos crates.

## Notas

Criado em 2026-11-23. Aguardando o spike.
