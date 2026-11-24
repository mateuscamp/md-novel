# 0002 — O estilo.yaml é a fonte única; toda propriedade sai explícita no CSS e no reference.docx

**Status:** Aceito
**Data:** 2026-11-23
**Driver ★:** Fidelidade de estilo editor ↔ .docx

## Contexto

O motivo do projeto é o autor ver na tela a mesma tipografia que o .docx
recebe. Com Zettlr, isso hoje são duas fontes de verdade (CSS próprio e
reference.docx editado no Word) que se desencontram. A decisão de só exportar
(ADR 0001) já está tomada; faltava definir como um arquivo de estilo vira os
dois lados sem que um deles ganhe ou perca propriedade em silêncio.

Quantum atual: uma ferramenta de linha de comando, três crates num workspace
Rust. O dado (estilo.yaml, capítulos .md) é do autor e fica na pasta dele.

## Alternativas consideradas

| Opção | Ganha | Perde | Por que não |
|---|---|---|---|
| Herança dentro do Word (`basedOn` Body Text) e cascata no CSS | arquivos menores | dois mecanismos de herança diferentes para conferir; padrão do navegador e padrão do Word entram onde nada foi declarado | a diferença aparece justamente onde ninguém olhou |
| Modelo do reference.docx embutido no binário | funciona sem Pandoc para gerar o reference | cópia de um arquivo do Pandoc (GPL) dentro do repositório; fica velho quando o Pandoc muda os estilos | o Pandoc já é obrigatório para exportar |
| Entrelinha como múltiplo do corpo da fonte (`lineRule="exact"`) | tela e .docx idênticos por construção | no Word aparece "Exatamente 18 pt", não "1,5 linhas", que é como editora e norma de submissão pedem | o .docx é lido por terceiros no Word |
| Uma propriedade por lado, com lista de "sem equivalente" ignorada | schema mais rico (viúvas, órfãs) | o autor declara algo que só um lado obedece | contraria o driver ★ |
| **Subconjunto comum, tudo explícito, modelo pedido ao Pandoc instalado** | cada propriedade resolvida no Rust e escrita por extenso nos dois lados; conferível por teste | schema pequeno; saídas maiores; exige Pandoc | — |

## Decisão

- O schema (`EstiloParagrafo`) só tem propriedade com tradução nos dois
  lados. Chave desconhecida no estilo.yaml é erro, nunca é ignorada.
- A herança (estilo nomeado parte do corpo) é resolvida no Rust. Cada regra
  CSS e cada estilo do Word recebe todas as propriedades por extenso.
- Os dois geradores desestruturam `EstiloParagrafo` sem `..`: propriedade
  nova não compila até ter tradução em `css.rs` e em `docx.rs`.
- O corpo vira o estilo "Body Text" do Pandoc. Estilo nomeado vira estilo de
  parágrafo com o mesmo nome, aplicado por `::: {custom-style="Nome"}`.
- O modelo do reference.docx vem de `pandoc --print-default-data-file
  reference.docx`, pedido por `crates/exportar`. `crates/estilo` recebe bytes
  e não abre processo.
- Entrelinha tem o sentido do Word (múltiplo da linha simples da fonte,
  `lineRule="auto"`). No CSS sai como
  `calc(var(--md-novel-linha-simples, 1.2) * N)`; o editor mede a fonte e
  define a variável.
- **As medidas do CSS são multiplicadas por `--md-novel-zoom`** (2026-11-24):
  tamanho da fonte, recuo e espaços saem como `calc(12pt * var(--md-novel-zoom, 1))`.
  É o zoom do editor do md Novel, que aproxima só o texto (ADR 0003). Sem a
  variável vale 1: o resultado é o de antes, na tela e em outro editor. O
  .docx não tem zoom e não muda.
- Espaço antes e depois vão em `padding-top`/`padding-bottom` no CSS, não em
  margem: margens de blocos vizinhos se fundem, e o Word soma as duas.
- A regra de estilo nomeado é
  `:is(<seletor do corpo>)[data-md-novel-estilo="Nome"]`. Como as duas regras
  trazem todas as propriedades, a nomeada precisa ser mais específica que a do
  corpo, seja qual for o seletor dele. O editor põe o atributo no mesmo
  elemento que casa com o seletor do corpo.
- Valor que um dos lados arredondaria de forma visível é rejeitado: tamanho
  de fonte fora da grade de 0.5pt do Word não entra.

## Consequências

**Ganha:** uma propriedade declarada aparece nos dois lados ou o build
quebra; o que vai para o Word é conferido no .docx que o Pandoc de fato
escreve.

**Perde:**
- Schema v1 só cobre parágrafo: fonte, tamanho, entrelinha, alinhamento, recuo
  de primeira linha, espaço antes e depois, itálico, negrito. Títulos, tamanho
  de página, margens e hifenização ficam com o padrão do Pandoc.
- Sem o editor medir a fonte, a entrelinha na tela usa 1.2 como linha simples,
  que é aproximação: a tela pode ficar mais apertada ou mais solta que o Word.
- O parágrafo logo depois de um título usa "First Paragraph", que herda do
  corpo, então sai com o mesmo recuo. Abrir capítulo sem recuo ainda não dá.
- Medida em `em` e recuo negativo são rejeitados.
- Recuo e espaçamento são arredondados para 1/20 de ponto no Word (1.25cm
  vira 709 twips, diferença de 0,006 mm) e ficam como escritos no CSS.
- A soma de espaço antes e depois no Word não foi conferida num Word real
  (não há Word nem LibreOffice na máquina de desenvolvimento); a tela soma,
  conferido no Chrome.
- `serde_yaml_ng` depende de `unsafe-libyaml`, marcada como sem manutenção
  (RUSTSEC-2024-0449, aviso, sem vulnerabilidade conhecida). Trocar por um
  leitor de YAML em Rust puro fica para quando houver um defeito concreto.
- Mesmas propriedades não quer dizer mesma página: métrica de fonte e
  hifenização diferem entre o navegador e o Word.

**Aceito conscientemente:** o Pandoc pode mudar o modelo (renomear "Body
Text", mudar a herança de "First Paragraph"). O gerador recusa modelo sem
`BodyText` e o teste de fidelidade reprova; revisitar quando isso acontecer ou
quando a primeira propriedade pedida não tiver tradução num dos lados.

## Governança

| O que verifica | Mecanismo | Onde roda | Falha quando |
|---|---|---|---|
| Fidelidade no .docx real | `crates/exportar/tests/fidelidade_pandoc.rs` | `make check` | o .docx que o Pandoc escreve não tem uma propriedade do estilo.yaml, ou ela não está no parágrafo certo |
| Subconjunto | `toda_chave_do_schema_sai_no_css_e_no_docx` em `crates/estilo/tests/compilador.rs` | `make check` | uma chave do schema falta num lado, ou aparece mais de uma vez num estilo |
| Tradução obrigatória | desestruturação sem `..` e `clippy::wildcard_enum_match_arm = "deny"` | compilação | propriedade ou variante nova sem tradução |
| Saída estável | goldens em `crates/estilo/tests/golden/` | `make check` | CSS ou XML gerado muda sem o golden mudar junto |
| `estilo` é Rust puro | `crates/estilo/tests/fronteira.rs` | `make check` | dependência fora da lista, `std::process` ou `std::net` em `crates/estilo` |

Cada checagem foi vista reprovar em 2026-11-23, com o código quebrado de
propósito (justificado trocado por esquerda, recuo removido de um lado,
dependência nova, processo em `estilo`).

A especificidade e a soma dos espaços foram conferidas uma vez no Chrome
(2026-11-23): com o seletor de corpo `.cm-content > .cm-line`, a regra antiga
mostrava a epígrafe como corpo; a nova a mostra à direita e em itálico, e duas
placas seguidas ficam a 28,8 + 16 + 16 px. Não há teste automático de
navegador: os testes conferem o texto do CSS.

Regra derivada no `AGENTS.md`: Propriedade nova de estilo = campo em
`EstiloParagrafo` e `Parcial` + tradução em `css.rs` e `docx.rs` + linha na
tabela do teste do subconjunto + goldens, no mesmo commit. Sem equivalente num
dos lados: rejeitar.

## Notas

Criado em 2026-11-23 · Aprovado em 2026-11-23 pelo autor.
A decisão de stack do editor (Tauri + CodeMirror 6) entra como 0003.
