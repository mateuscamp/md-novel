//! Spike 0003: janela mínima para medir o CodeMirror 6 no WebView do sistema.

/// Linha de medida vinda da página, impressa no terminal do `cargo tauri dev`.
#[tauri::command]
fn registrar_medida(linha: String) {
    println!("[medida] {linha}");
}

fn main() {
    // O critério do ADR 0003 exige aceleração ligada: avisar se alguma
    // variável conhecida a desligou neste processo.
    for variavel in [
        "WEBKIT_DISABLE_DMABUF_RENDERER",
        "WEBKIT_DISABLE_COMPOSITING_MODE",
        "LIBGL_ALWAYS_SOFTWARE",
    ] {
        if let Ok(valor) = std::env::var(variavel) {
            println!("[medida] atenção: {variavel}={valor} está definida");
        }
    }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![registrar_medida])
        .run(tauri::generate_context!())
        .unwrap_or_else(|erro| panic!("a janela do spike não abriu: {erro}"));
}
