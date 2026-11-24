#!/bin/sh
# Põe o md Novel no menu de aplicativos do Linux, e tira de lá. É o que o
# `make instalar` e o `make desinstalar` chamam (ADR 0003).
#
#   instalar.sh conferir <prefixo>
#   instalar.sh instalar <prefixo> <programa compilado>
#   instalar.sh desinstalar <prefixo>
#
# São três arquivos dentro do prefixo (o padrão do Makefile é ~/.local), e
# mais nenhum: o programa, o ícone e a entrada do menu. Desinstalar apaga
# esses três pelo caminho exato e não mexe nas pastas nem no que o app
# guardou (a escolha de tema fica em ~/.local/share/<identificador>).
set -eu

# O mesmo identificador do tauri.conf.json: `tests/instalar.rs` confere.
IDENTIFICADOR=io.github.mateuscamp.md-novel

falhar() {
  printf 'erro: %s\n' "$1" >&2
  exit 1
}

# Guardado aqui, e não expandido dentro de `uso`: com `set -u`, um `sh` antigo
# (o do Mac é o bash 3.2) trata "$@" vazio como variável que não existe.
if [ $# -eq 0 ]; then
  recebido="nada"
else
  recebido=$*
fi

uso() {
  falhar "uso: instalar.sh conferir <prefixo> | instalar.sh instalar <prefixo> <programa compilado> | instalar.sh desinstalar <prefixo>; recebi: $recebido"
}

# Só no Linux: no Mac o menu é outro (um pacote .app), que ainda não existe.
conferir_sistema() {
  sistema=$(uname -s)
  [ "$sistema" = Linux ] ||
    falhar "instalar no menu só existe no Linux por enquanto, e este sistema é \"$sistema\"; abra o programa pelo terminal: target/release/md-novel-app <pasta do livro>"
}

# O caminho do programa vai entre aspas na linha Exec da entrada do menu. A
# especificação reserva ali dentro as aspas, a crase, o cifrão e a barra
# invertida, usa % para os códigos de campo e não aceita = no nome do
# programa. Em vez de escapar, recusa: um prefixo assim é quase sempre engano.
conferir_prefixo() {
  case "$1" in
    /*) ;;
    *) falhar "o prefixo precisa ser um caminho inteiro, como /home/voce/.local; recebi \"$1\"" ;;
  esac
  case "$1" in
    *'"'* | *'`'* | *'$'* | *'\'* | *'%'* | *'='*)
      falhar "o prefixo não pode ter aspas, crase, \$, \\, % nem =, que a entrada do menu reserva; recebi \"$1\""
      ;;
  esac
  # Quebra de linha, tabulação e outros caracteres de controle. O `x` no fim
  # guarda a quebra de linha final, que o `$( )` cortaria.
  sem_controle=$(printf '%sx' "$1" | LC_ALL=C tr -d '[:cntrl:]')
  [ "$sem_controle" = "${1}x" ] ||
    falhar "o prefixo não pode ter quebra de linha nem outro caractere de controle; recebi \"$1\""
}

# Cada arquivo é copiado para um temporário na pasta de destino (`preparar`)
# e só depois trocado de uma vez (`publicar`). Os três são preparados antes
# de o primeiro ser publicado: disco cheio ou pasta sem permissão aparece
# antes de qualquer coisa mudar. E quem está com o programa aberto continua
# com o antigo, porque o arquivo dele não é reescrito, é substituído.
temporarios=""
limpar() {
  # Os caminhos não têm quebra de linha (o prefixo foi conferido).
  printf '%s' "$temporarios" | while IFS= read -r temporario; do
    [ -z "$temporario" ] || rm -f "$temporario"
  done
}
trap limpar EXIT
# Interrompido (Ctrl-C, terminal fechado): sai pelo caminho comum, para a
# limpeza acima rodar em qualquer `sh`.
trap 'exit 1' HUP INT TERM

preparar() {
  origem=$1
  destino=$2
  modo=$3
  temporario="$destino.instalando.$$"
  temporarios="$temporarios$temporario
"
  mkdir -p "$(dirname "$destino")" && cp "$origem" "$temporario" && chmod "$modo" "$temporario" ||
    falhar "não consegui preparar \"$destino\""
}

publicar() {
  mv -f "$1.instalando.$$" "$1" || falhar "não consegui gravar \"$1\""
}

[ $# -ge 2 ] || uso
comando=$1
prefixo=$2

case "$comando" in
  conferir | desinstalar) [ $# -eq 2 ] || uso ;;
  instalar) [ $# -eq 3 ] || uso ;;
  *) uso ;;
esac

conferir_sistema
conferir_prefixo "$prefixo"

programa="$prefixo/bin/md-novel-app"
icone="$prefixo/share/icons/hicolor/64x64/apps/$IDENTIFICADOR.png"
entrada="$prefixo/share/applications/$IDENTIFICADOR.desktop"

case "$comando" in
  conferir) ;;

  instalar)
    compilado=$3
    icone_do_projeto="$(dirname "$0")/../src-tauri/icons/icon.png"
    [ -f "$compilado" ] ||
      falhar "não achei o programa compilado em \"$compilado\"; compile antes: cd app && npm ci --ignore-scripts && cargo tauri build --no-bundle"
    [ -f "$icone_do_projeto" ] ||
      falhar "não achei o ícone do projeto em \"$icone_do_projeto\""

    # O ícone vai pelo caminho, não pelo nome: assim aparece mesmo com um
    # prefixo fora das pastas de ícone do sistema e sem atualizar cache.
    # StartupWMClass é a classe que a janela anuncia (o nome do binário): é
    # por ela que o painel junta a janela aberta com esta entrada.
    rascunho=$(mktemp "${TMPDIR:-/tmp}/md-novel-entrada.XXXXXX") ||
      falhar "não consegui criar um arquivo temporário para a entrada do menu"
    temporarios="$temporarios$rascunho
"
    {
      printf '[Desktop Entry]\n'
      printf 'Type=Application\n'
      printf 'Name=md Novel\n'
      printf 'Comment=Escrever um livro em Markdown, com o estilo do .docx na tela\n'
      printf 'Exec="%s"\n' "$programa"
      printf 'Icon=%s\n' "$icone"
      printf 'Terminal=false\n'
      printf 'Categories=Office;WordProcessor;\n'
      printf 'StartupWMClass=md-novel-app\n'
    } >"$rascunho" || falhar "não consegui escrever a entrada do menu em \"$rascunho\""

    preparar "$compilado" "$programa" 755
    preparar "$icone_do_projeto" "$icone" 644
    preparar "$rascunho" "$entrada" 644
    # A entrada do menu por último: ela nunca aponta para um programa que
    # ainda não chegou.
    publicar "$programa"
    publicar "$icone"
    publicar "$entrada"

    printf 'md Novel instalado: procure por "md Novel" no menu de aplicativos.\n'
    printf '  programa: %s\n  entrada do menu: %s\n  ícone: %s\n' "$programa" "$entrada" "$icone"
    ;;

  desinstalar)
    rm -f "$programa" "$entrada" "$icone"
    printf 'md Novel desinstalado de %s. O que o app guardou (a escolha de tema) não foi apagado.\n' "$prefixo"
    ;;
esac
