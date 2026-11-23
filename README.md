# md-novel

Um estilo só para o livro inteiro. Você escreve os capítulos em Markdown, um
arquivo por capítulo, e descreve a tipografia uma vez num `estilo.yaml`: fonte,
tamanho, entrelinha, alinhamento, recuo e espaços. Desse arquivo saem o CSS do
editor e o `reference.docx` do Pandoc, de modo que a tela e o .docx recebem as
mesmas propriedades.

**Estado:** ferramenta de linha de comando. O editor próprio ainda não existe;
enquanto isso, o CSS gerado serve no Zettlr.

## Instalação

Precisa do Rust (pelo [rustup](https://rustup.rs)) e do Pandoc 3 (testado com
o 3.11).

No Arch Linux:

```bash
sudo pacman -S rustup pandoc-cli
```

```bash
rustup default stable
```

No macOS, instale o Rust pelo [rustup.rs](https://rustup.rs) e o Pandoc pelo
Homebrew:

```bash
brew install pandoc
```

Depois, de dentro desta pasta:

```bash
cargo install --locked --path crates/cli
```

O comando `md-novel` fica em `~/.cargo/bin`.

## Uso

Copie [`exemplos/estilo.yaml`](exemplos/estilo.yaml) para a pasta do livro e
ajuste. Cada chave está comentada no próprio arquivo.

**Gerar o CSS e o reference.docx:**

```bash
md-novel estilo estilo.yaml --css editor.css --docx reference.docx
```

**Exportar o livro para .docx:**

```bash
md-novel exportar --estilo estilo.yaml --saida livro.docx 01-abertura.md 02-a-casa.md
```

Os capítulos entram na ordem em que você os passa. Com nomes numerados,
`md-novel exportar --estilo estilo.yaml --saida livro.docx 0*.md` já sai na
ordem. Imagens são procuradas a partir da pasta de cada capítulo.

**Estilos nomeados** (epígrafe, carta, separador) vão no capítulo assim:

```markdown
::: {custom-style="Epígrafe"}
A pele guarda o que a memória solta.
:::
```

No .docx, o parágrafo sai com o estilo de mesmo nome, que você também encontra
na lista de estilos do Word.

## Com o Zettlr

1. Gere o CSS com o seletor de linha do editor do Zettlr:

   ```bash
   md-novel estilo estilo.yaml --css zettlr.css --docx reference.docx --seletor-corpo ".cm-content .cm-line"
   ```

2. No Zettlr, abra o gerenciador de recursos (Assets Manager), na aba de CSS
   personalizado, e cole o conteúdo de `zettlr.css`.
3. Para exportar, use o `md-novel exportar`, ou aponte o perfil de exportação
   para Word do Zettlr para o `reference.docx` gerado, com a chave
   `reference-doc`.

Gere de novo e cole de novo sempre que mudar o `estilo.yaml`.

No Zettlr há limites que o editor próprio vai resolver:

- Os estilos nomeados não aparecem na tela, só no .docx: o Zettlr não marca os
  blocos `:::`.
- Recuo e justificação valem para toda linha, inclusive títulos e linhas em
  branco.
- O seletor `.cm-content .cm-line` foi tirado da documentação do Zettlr 3 e não
  foi conferido numa instalação; se nada mudar na tela, veja a classe da linha
  nas ferramentas de desenvolvedor do Zettlr.

## O que esperar

- **Mesmas propriedades, não a mesma página.** O navegador e o Word medem as
  fontes e hifenizam de jeitos diferentes.
- **Entrelinha aproximada na tela.** O Word mede a linha simples pela fonte; o
  CSS usa 1,2 vez o tamanho, a menos que o editor defina
  `--md-novel-linha-simples`.
- **Só de ida.** O md-novel não lê de volta o .docx revisado pela editora: as
  mudanças voltam para os .md à mão ([ADR 0001](docs/adr/0001-exportar-so-de-ida-md-para-docx.md)).
- **Fica de fora, por enquanto:** títulos, tamanho de página, margens e
  hifenização usam o padrão do Pandoc
  ([ADR 0002](docs/adr/0002-estilo-yaml-fonte-unica-com-propriedades-explicitas.md)).

## Desenvolvimento

```bash
make check
```

Formata, roda o clippy e a suíte inteira; precisa do `pandoc` no PATH. O CI
roda o mesmo comando no Linux e no Mac M1. Regras do repositório em
[AGENTS.md](AGENTS.md), decisões em [docs/adr](docs/adr/README.md).
