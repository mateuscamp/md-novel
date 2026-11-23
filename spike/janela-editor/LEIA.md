# Spike 0003: janela do editor

Experimento descartável do [ADR 0003](../../docs/adr/0003-editor-desktop-em-tauri-com-codemirror-6.md).
Mora só na branch `spike/0003-janela-editor` e não é mergeado.

Responde: o WebKitGTK (Linux) e o WKWebView (Mac) desenham um capítulo de 10
mil palavras com o CSS do `exemplos/estilo.yaml` com latência e rolagem
dentro do critério do ADR?

## Rodar

Precisa de Node, Rust, `cargo tauri` (tauri-cli 2) e, no Linux, `webkit2gtk-4.1`.

```bash
cd spike/janela-editor && npm ci --ignore-scripts && cargo tauri build --debug --no-bundle
```

```bash
./src-tauri/target/debug/md-novel-spike-janela
```

No Hyprland, `python3 -I medir.py ./src-tauri/target/debug/md-novel-spike-janela captura.png`
roda, espera as medidas e fotografa só a janela.

Ao abrir, a janela mede sozinha (inserções no meio do texto e rolagem) e
escreve o resultado no terminal e no canto da janela. Depois, digitar no
texto mede a latência das teclas de verdade; a cada 50 teclas sai uma linha
nova.
