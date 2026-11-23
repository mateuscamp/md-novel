# 0001 — O app só exporta de .md para .docx; o .docx revisado não volta

**Status:** Aceito
**Data:** 2026-11-23
**Driver ★:** Fidelidade de estilo editor ↔ .docx

## Contexto

O manuscrito vive em .md soltos numa pasta (inclusive no Dropbox), e o .docx é
gerado pelo Pandoc com um reference.docx tirado do estilo.yaml. A editora
devolve o .docx com controle de alterações e comentários. Antes de escolher a
stack, era preciso saber se esse .docx precisa voltar automaticamente para os
.md: a volta perde informação e é ela que torna o projeto difícil.

Quantum atual: nenhum código ainda. Previsto: um app desktop; os dados (*.md,
livro.yaml, estilo.yaml) pertencem ao autor e ficam na pasta dele.

## Alternativas consideradas

| Opção | Ganha | Perde | Por que não |
|---|---|---|---|
| Ida e volta pelo Pandoc (`-f docx+styles --track-changes=all`) e diff para aceitar nos .md | revisão da editora entra sem redigitar | ruído de formatação, comentários sem lugar no .md, livro exportado como arquivo único que precisa ser fatiado de volta por capítulo | custo de um segundo produto, sem volume de revisão medido que o justifique |
| Decidir depois do primeiro ciclo com a editora | decisão com dado real | escopo indefinido enquanto se constrói | o autor decidiu já |
| Só ida; a volta é manual | escopo menor; o .md nunca é reescrito por conversão | o autor relê o .docx revisado e aplica as mudanças à mão | — |

## Decisão

O app converte só de .md para .docx. O autor lê o .docx revisado no Word ou no
LibreOffice e edita os .md. Isso mantém a regra de que o app nunca reformata o
.md do autor: nenhum texto entra nos capítulos por conversão automática.

## Consequências

**Ganha:** o compilador de estilo e a exportação ficam num único sentido, com
fidelidade testável; o .md continua sendo escrito só pelo autor.

**Perde:** cada rodada de revisão da editora custa trabalho manual de aplicar
as alterações capítulo a capítulo, sem ferramenta que mostre o que falta.

**Aceito conscientemente:** se uma rodada de revisão trouxer alterações demais
para aplicar à mão (o autor julga no primeiro ciclo real), um ADR novo avalia a
importação com `-f docx+styles --track-changes=all` e substitui este.

## Governança

| O que verifica | Mecanismo | Onde roda | Falha quando |
|---|---|---|---|
| Nenhum caminho de importação de .docx | teste de arquitetura em `crates/exportar` que procura `docx` como formato de entrada (`-f docx`, `--from=docx`, `--track-changes`) nos argumentos montados para o Pandoc | CI, a partir do primeiro commit de `crates/exportar` | algum argumento ou fonte usa docx como formato de entrada |
| Até existir código | revisão manual na /code-audit de cada diff que toque o Pandoc | a cada PR | diff importa .docx sem ADR que substitua este |

Regra derivada no `AGENTS.md`: Pandoc só converte de .md para .docx. Importar
.docx exige um ADR que substitua o 0001.

## Notas

Criado em 2026-11-23 · Aprovado em 2026-11-23 pelo autor.
A decisão de stack (Tauri + CodeMirror 6, proposta no debate como "0001")
entra como 0002 quando for registrada.
