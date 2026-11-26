#!/bin/sh
# Põe o md Novel como aplicativo de desktop no macOS (`md Novel.app`), e tira
# de lá. É o que o `make instalar` e o `make desinstalar` chamam no Mac
# (ADR 0003).
#
#   instalar.sh conferir <pasta>
#   instalar.sh instalar <pasta> <programa compilado>
#   instalar.sh desinstalar <pasta>
#
# O pacote `md Novel.app` nasce inteiro num temporário dentro da pasta de
# destino (o padrão do Makefile é ~/Applications) antes de ser publicado.
# Desinstalar apaga só esse pacote, depois de conferir o identificador no
# Info.plist, e não mexe nos outros aplicativos nem no que o app guardou
# (~/Library/Application Support/io.github.mateuscamp.md-novel).
set -eu

# O mesmo identificador e a mesma versão do tauri.conf.json:
# `tests/instalar_macos.rs` confere ambos.
IDENTIFICADOR=io.github.mateuscamp.md-novel
VERSAO=0.1.0
PACOTE="md Novel.app"

falhar() {
  printf 'erro: %s\n' "$1" >&2
  exit 1
}

if [ $# -eq 0 ]; then
  recebido="nada"
else
  recebido=$*
fi

uso() {
  falhar "uso: instalar.sh conferir <pasta> | instalar.sh instalar <pasta> <programa compilado> | instalar.sh desinstalar <pasta>; recebi: $recebido"
}

conferir_sistema() {
  sistema=$(uname -s)
  [ "$sistema" = Darwin ] ||
    falhar "instalar o pacote .app só existe no macOS (Darwin), e este sistema é \"$sistema\""
}

conferir_prefixo() {
  case "$1" in
    /*) ;;
    *) falhar "o prefixo precisa ser um caminho inteiro, como $HOME/Applications; recebi \"$1\"" ;;
  esac
  sem_barras=$(printf '%s' "$1" | tr -d '/')
  [ -n "$sem_barras" ] ||
    falhar "o prefixo não pode ser a raiz \"/\"; recebi \"$1\""
  sem_controle=$(printf '%sx' "$1" | LC_ALL=C tr -d '[:cntrl:]')
  [ "$sem_controle" = "${1}x" ] ||
    falhar "o prefixo não pode ter quebra de linha nem outro caractere de controle; recebi \"$1\""
}

conferir_ferramentas() {
  command -v sips >/dev/null 2>&1 ||
    falhar "não achei o sips no PATH; ele vem com o macOS para gerar o ícone do aplicativo"
  command -v iconutil >/dev/null 2>&1 ||
    falhar "não achei o iconutil no PATH; ele vem com o macOS para gerar o ícone do aplicativo"
}

eh_nosso_pacote() {
  conteudo="$1/Contents"
  plist="$conteudo/Info.plist"
  [ -d "$conteudo" ] && [ ! -L "$conteudo" ] &&
    [ -f "$plist" ] && [ ! -L "$plist" ] &&
    grep -Fq "<string>$IDENTIFICADOR</string>" "$plist"
}

conferir_destino_seguro() {
  if [ -L "$1" ]; then
    falhar "recusei mexer em \"$1\" porque é um link simbólico"
  fi
  if [ -e "$1" ] && [ ! -d "$1" ]; then
    falhar "recusei mexer em \"$1\" porque já existe um arquivo comum com esse nome"
  fi
  if [ -d "$1" ] && ! eh_nosso_pacote "$1"; then
    falhar "recusei mexer em \"$1\" porque não parece um pacote do md Novel ($IDENTIFICADOR)"
  fi
}

trabalho=""
staging=""
limpar() {
  [ -z "$trabalho" ] || rm -rf "$trabalho"
  [ -z "$staging" ] || rm -rf "$staging"
}
trap limpar EXIT
trap 'exit 1' HUP INT TERM

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

while :; do
  case "$prefixo" in
    */) prefixo=${prefixo%/} ;;
    *) break ;;
  esac
done

destino="$prefixo/$PACOTE"

case "$comando" in
  conferir)
    conferir_ferramentas
    conferir_destino_seguro "$destino"
    ;;

  instalar)
    conferir_ferramentas
    compilado=$3
    icone_fonte="$(dirname "$0")/../src-tauri/icons/icon-source.png"
    [ -f "$compilado" ] ||
      falhar "não achei o programa compilado em \"$compilado\"; compile antes: cd app && npm ci --ignore-scripts && cargo tauri build --no-bundle"
    [ -f "$icone_fonte" ] ||
      falhar "não achei o ícone do projeto em \"$icone_fonte\""

    conferir_destino_seguro "$destino"

    tmp_base=${TMPDIR:-/tmp}
    tmp_base=${tmp_base%/}
    trabalho=$(mktemp -d "$tmp_base/md-novel-macos.XXXXXX") ||
      falhar "não consegui criar pasta temporária para montar o ícone"
    iconset="$trabalho/icon.iconset"
    icns="$trabalho/icon.icns"
    mkdir -p "$iconset" || falhar "não consegui criar \"$iconset\""

    gerar_tamanho() {
      sips -z "$1" "$1" "$icone_fonte" --out "$iconset/$2" >/dev/null ||
        falhar "não consegui redimensionar o ícone para $1x$1"
    }
    gerar_tamanho 16 icon_16x16.png
    gerar_tamanho 32 icon_16x16@2x.png
    gerar_tamanho 32 icon_32x32.png
    gerar_tamanho 64 icon_32x32@2x.png
    gerar_tamanho 128 icon_128x128.png
    gerar_tamanho 256 icon_128x128@2x.png
    gerar_tamanho 256 icon_256x256.png
    gerar_tamanho 512 icon_256x256@2x.png
    gerar_tamanho 512 icon_512x512.png
    gerar_tamanho 1024 icon_512x512@2x.png

    iconutil -c icns "$iconset" -o "$icns" ||
      falhar "não consegui gerar o ícone .icns com iconutil"

    mkdir -p "$prefixo" || falhar "não consegui criar a pasta de aplicativos \"$prefixo\""
    # Pasta temporária exclusiva no mesmo sistema de arquivos: o pacote nasce
    # já com o nome final `md Novel.app` (necessário para o `codesign` tratar
    # como bundle) e só entra em `$destino` depois de pronto e assinado.
    staging=$(mktemp -d "$prefixo/.md-novel-instalando.XXXXXX") ||
      falhar "não consegui criar pasta temporária em \"$prefixo\""
    temporario="$staging/$PACOTE"
    mkdir -p "$temporario/Contents/MacOS" "$temporario/Contents/Resources" &&
      chmod 755 "$temporario" ||
      falhar "não consegui preparar \"$temporario\""

    cp "$compilado" "$temporario/Contents/MacOS/md-novel-app" &&
      chmod 755 "$temporario/Contents/MacOS/md-novel-app" ||
      falhar "não consegui copiar o programa para \"$temporario\""

    cp "$icns" "$temporario/Contents/Resources/icon.icns" &&
      chmod 644 "$temporario/Contents/Resources/icon.icns" ||
      falhar "não consegui copiar o ícone para \"$temporario\""

    {
      printf '<?xml version="1.0" encoding="UTF-8"?>\n'
      printf '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n'
      printf '<plist version="1.0">\n'
      printf '<dict>\n'
      printf '  <key>CFBundleDevelopmentRegion</key>\n'
      printf '  <string>pt-BR</string>\n'
      printf '  <key>CFBundleDisplayName</key>\n'
      printf '  <string>md Novel</string>\n'
      printf '  <key>CFBundleExecutable</key>\n'
      printf '  <string>md-novel-app</string>\n'
      printf '  <key>CFBundleIconFile</key>\n'
      printf '  <string>icon</string>\n'
      printf '  <key>CFBundleIdentifier</key>\n'
      printf '  <string>%s</string>\n' "$IDENTIFICADOR"
      printf '  <key>CFBundleInfoDictionaryVersion</key>\n'
      printf '  <string>6.0</string>\n'
      printf '  <key>CFBundleName</key>\n'
      printf '  <string>md Novel</string>\n'
      printf '  <key>CFBundlePackageType</key>\n'
      printf '  <string>APPL</string>\n'
      printf '  <key>CFBundleShortVersionString</key>\n'
      printf '  <string>%s</string>\n' "$VERSAO"
      printf '  <key>CFBundleVersion</key>\n'
      printf '  <string>%s</string>\n' "$VERSAO"
      printf '  <key>LSMinimumSystemVersion</key>\n'
      printf '  <string>10.15</string>\n'
      printf '  <key>NSHighResolutionCapable</key>\n'
      printf '  <true/>\n'
      printf '</dict>\n'
      printf '</plist>\n'
    } >"$temporario/Contents/Info.plist" &&
      chmod 644 "$temporario/Contents/Info.plist" ||
      falhar "não consegui gravar o Info.plist em \"$temporario\""

    if command -v codesign >/dev/null 2>&1; then
      codesign --force --deep --sign - "$temporario" >/dev/null ||
        falhar "não consegui assinar o pacote \"$temporario\""
    fi

    if [ -d "$destino" ]; then
      anterior=$(mktemp -d "$prefixo/.md-novel-anterior.XXXXXX") ||
        falhar "não consegui reservar pasta temporária para a instalação anterior"
      mv "$destino" "$anterior/$PACOTE" || {
        rm -rf "$anterior"
        falhar "não consegui afastar a instalação anterior em \"$destino\""
      }
      if mv "$temporario" "$destino"; then
        rm -rf "$anterior"
      else
        if mv "$anterior/$PACOTE" "$destino"; then
          rmdir "$anterior" || true
        fi
        falhar "não consegui publicar o novo pacote em \"$destino\""
      fi
    else
      mv "$temporario" "$destino" ||
        falhar "não consegui publicar o pacote em \"$destino\""
    fi

    printf 'md Novel instalado: procure por "md Novel" no Spotlight ou abra em %s.\n' "$destino"
    ;;

  desinstalar)
    conferir_destino_seguro "$destino"
    if [ -d "$destino" ]; then
      rm -rf "$destino"
    fi
    printf 'md Novel desinstalado de %s. O que o app guardou não foi apagado.\n' "$prefixo"
    ;;
esac
