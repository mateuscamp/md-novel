//! md-novel, o programa de desktop. Adaptador fino: cada comando repassa
//! para a `Sessao`, que é quem conhece o livro. Sem regra de estilo aqui.

mod exportacao;
mod sessao;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Window, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use workspace::Gravacao;

use crate::exportacao::ExportacaoFeita;
use crate::sessao::{ErroDeComando, LivroAberto, Sessao};

#[derive(Default)]
struct Estado {
    sessao: Mutex<Sessao>,
    /// Já houve um pedido de fechar que a página ainda não atendeu.
    fechamento_pedido: AtomicBool,
}

impl Estado {
    /// Um comando que entrou em pânico não pode travar o app para sempre: a
    /// sessão continua utilizável, e o que estava no disco continua lá.
    fn sessao(&self) -> MutexGuard<'_, Sessao> {
        self.sessao.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// O livro aberto ao iniciar (pasta passada na linha de comando), se houver.
/// `async` porque montar a lista lê todos os capítulos: fora da thread
/// principal, a janela não trava enquanto isso.
#[tauri::command(async)]
fn livro_atual(estado: State<'_, Estado>) -> Result<Option<LivroAberto>, ErroDeComando> {
    estado.sessao().livro_atual()
}

/// Mostra o seletor de pasta do sistema e abre a pasta escolhida. A página
/// não manda caminho nenhum: quem escolhe é o autor, na janela do sistema.
/// `async` aqui tira o comando da thread principal, que o diálogo bloquearia.
#[tauri::command(async)]
fn escolher_pasta(
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<Option<LivroAberto>, ErroDeComando> {
    let Some(escolhida) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let pasta = escolhida.into_path().map_err(|erro| ErroDeComando {
        codigo: "erro",
        mensagem: format!("a pasta escolhida não é um caminho deste computador: {erro}"),
    })?;
    estado.sessao().abrir_pasta(&pasta).map(Some)
}

/// Exporta o livro aberto para .docx. O destino vem do seletor de "salvar
/// como" do sistema; `None` se o autor cancelou. A sessão só fica travada
/// para tirar o plano: o Pandoc roda com ela livre.
#[tauri::command(async)]
fn exportar_livro(
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<Option<ExportacaoFeita>, ErroDeComando> {
    let plano = estado.sessao().plano_de_exportacao()?;
    let destino = match destino_de_teste() {
        Some(destino) => destino,
        None => {
            let escolhido = app
                .dialog()
                .file()
                .set_title("Exportar o livro para .docx")
                .add_filter("Documento do Word", &["docx"])
                .set_directory(&plano.pasta)
                .set_file_name(format!("{}.docx", plano.nome))
                .blocking_save_file();
            let Some(escolhido) = escolhido else {
                return Ok(None);
            };
            escolhido.into_path().map_err(|erro| ErroDeComando {
                codigo: "erro",
                mensagem: format!("o destino escolhido não é um caminho deste computador: {erro}"),
            })?
        }
    };
    exportacao::exportar(&plano, &destino).map(Some)
}

/// Destino fixo para o teste funcional, no lugar do seletor do sistema. Só
/// existe em build de depuração.
fn destino_de_teste() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        std::env::var_os("MD_NOVEL_TESTE_EXPORTAR").map(PathBuf::from)
    } else {
        None
    }
}

#[tauri::command]
fn abrir_capitulo(nome: String, estado: State<'_, Estado>) -> Result<String, ErroDeComando> {
    estado.sessao().abrir_capitulo(&nome)
}

/// Devolve `true` se gravou e `false` se o texto já era o do disco.
#[tauri::command]
fn salvar_capitulo(
    nome: String,
    texto: String,
    estado: State<'_, Estado>,
) -> Result<bool, ErroDeComando> {
    let gravacao = estado.sessao().salvar_capitulo(&nome, &texto)?;
    Ok(gravacao == Gravacao::Gravado)
}

/// Linha de diagnóstico da interface, impressa no terminal de quem abriu o app.
#[tauri::command]
fn registrar(linha: String) {
    println!("[interface] {linha}");
}

/// O que a interface faz sozinha no teste funcional (`app/teste_funcional.py`).
#[derive(Serialize)]
struct RoteiroDeTeste {
    /// Texto digitado no fim do primeiro capítulo.
    digitar: String,
    /// Pedir para fechar a janela logo depois de digitar.
    fechar: bool,
    /// Exportar o livro logo depois de digitar.
    exportar: bool,
    /// Tema para esta janela (`claro` ou `escuro`), sem guardar a escolha.
    tema: Option<String>,
    /// Quantas vezes clicar no botão de tema, como o autor faria.
    cliques_no_tema: u32,
    /// Entrar no modo foco depois de digitar.
    foco: bool,
}

/// O roteiro vem de variáveis de ambiente e só existe em build de depuração.
#[tauri::command]
fn roteiro_de_teste() -> Option<RoteiroDeTeste> {
    if !cfg!(debug_assertions) {
        return None;
    }
    Some(RoteiroDeTeste {
        digitar: std::env::var("MD_NOVEL_TESTE_DIGITAR").ok()?,
        fechar: std::env::var_os("MD_NOVEL_TESTE_FECHAR").is_some(),
        exportar: std::env::var_os("MD_NOVEL_TESTE_EXPORTAR").is_some(),
        tema: std::env::var("MD_NOVEL_TESTE_TEMA").ok(),
        cliques_no_tema: std::env::var("MD_NOVEL_TESTE_CLIQUES_NO_TEMA")
            .ok()
            .and_then(|cliques| cliques.parse().ok())
            .unwrap_or(0),
        foco: std::env::var_os("MD_NOVEL_TESTE_FOCO").is_some(),
    })
}

/// Pede para fechar a janela como o gerenciador de janelas faria: o Tauri
/// dispara o mesmo `CloseRequested`. Só em build de depuração, para o teste
/// funcional não depender de um gerenciador de janelas específico.
#[tauri::command]
fn teste_pedir_para_fechar(janela: Window) -> Result<(), ErroDeComando> {
    if !cfg!(debug_assertions) {
        return Ok(());
    }
    janela.close().map_err(|erro| ErroDeComando {
        codigo: "erro",
        mensagem: format!("o pedido de fechar não saiu: {erro}"),
    })
}

/// Fecha a janela de vez. A página chama depois de salvar o que faltava.
#[tauri::command]
fn fechar_janela(janela: Window, estado: State<'_, Estado>) -> Result<(), ErroDeComando> {
    estado.fechamento_pedido.store(false, Ordering::SeqCst);
    janela.destroy().map_err(|erro| ErroDeComando {
        codigo: "erro",
        mensagem: format!("a janela não fechou: {erro}"),
    })
}

/// O autor pediu para fechar. Na primeira vez a janela fica, e a página é
/// avisada para salvar e fechar por `fechar_janela`. Se ele pedir de novo
/// (a página travou, ou há um conflito que ele decidiu abandonar), fecha.
fn ao_pedir_para_fechar(janela: &Window, evento: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = evento else {
        return;
    };
    let ja_pedido = janela
        .state::<Estado>()
        .fechamento_pedido
        .swap(true, Ordering::SeqCst);
    if ja_pedido {
        return;
    }
    // Se a página não puder ser avisada, não há quem salve: fecha.
    if janela.emit("pedido-de-fechar", ()).is_ok() {
        api.prevent_close();
    }
}

fn main() {
    let estado = Estado::default();
    // `md-novel-app <pasta>` abre direto o livro daquela pasta.
    if let Some(pasta) = std::env::args_os().nth(1).map(PathBuf::from) {
        if let Err(erro) = estado.sessao().abrir_pasta(&pasta) {
            eprintln!("erro: {}", erro.mensagem);
        }
    }
    let resultado = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(estado)
        .on_window_event(ao_pedir_para_fechar)
        .invoke_handler(tauri::generate_handler![
            livro_atual,
            escolher_pasta,
            abrir_capitulo,
            salvar_capitulo,
            exportar_livro,
            registrar,
            roteiro_de_teste,
            teste_pedir_para_fechar,
            fechar_janela
        ])
        .run(tauri::generate_context!());
    if let Err(erro) = resultado {
        eprintln!("erro: a janela do md-novel não abriu: {erro}");
        std::process::exit(1);
    }
}
