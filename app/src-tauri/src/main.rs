//! md Novel, o programa de desktop. Adaptador fino: cada comando repassa
//! para a `Sessao`, que é quem conhece o livro. Sem regra de estilo aqui.

// Só no Linux: no Mac o WebView do sistema corrige sem o app pedir.
#[cfg(target_os = "linux")]
mod corretor;
mod exportacao;
mod fontes;
mod memoria;
mod sessao;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Window, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use workspace::Gravacao;

use crate::exportacao::ExportacaoFeita;
use crate::fontes::FaceDeFonte;
use crate::memoria::LivroLembrado;
use crate::sessao::{
    BuscaNoLivro, CapituloCriado, ErroDeComando, EstiloNoPainel, LivroAberto, LivroRenomeado,
    Sessao,
};

#[derive(Default)]
struct Estado {
    sessao: Mutex<Sessao>,
    /// Já houve um pedido de fechar que a página ainda não atendeu.
    fechamento_pedido: AtomicBool,
    corretor: Mutex<EstadoDoCorretor>,
}

/// O que o corretor ortográfico carregou ao abrir o app.
#[derive(Clone, Default, Serialize)]
struct EstadoDoCorretor {
    /// As línguas com dicionário carregado. Vazio fora do Linux, onde o
    /// WebView do sistema corrige por conta própria.
    linguas: Vec<String>,
    /// O que dizer ao autor se o dicionário da língua do app não está lá.
    aviso: Option<String>,
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

/// Cria o `estilo.yaml` do livro aberto, a pedido do autor (o botão do aviso
/// da pasta sem estilo). A página não manda nada: nem caminho, nem conteúdo.
#[tauri::command(async)]
fn criar_estilo(estado: State<'_, Estado>) -> Result<LivroAberto, ErroDeComando> {
    estado.sessao().criar_estilo()
}

/// O estilo do livro para o painel "Estilo" mostrar. Só lê.
#[tauri::command(async)]
fn estilo_para_editar(estado: State<'_, Estado>) -> Result<EstiloNoPainel, ErroDeComando> {
    estado.sessao().estilo_para_editar()
}

/// Grava o estilo que o painel mostra. A página manda os valores dos campos,
/// nunca um caminho nem o texto do arquivo: quem valida e escreve o
/// `estilo.yaml` é o Rust.
#[tauri::command(async)]
fn salvar_estilo(
    painel: EstiloNoPainel,
    estado: State<'_, Estado>,
) -> Result<LivroAberto, ErroDeComando> {
    estado.sessao().salvar_estilo(painel)
}

/// Cria um capítulo vazio no livro aberto. A página manda o nome que o autor
/// digitou, e só o nome: quem confere que ele não sai da pasta é o `workspace`.
#[tauri::command(async)]
fn criar_capitulo(
    nome: String,
    estado: State<'_, Estado>,
) -> Result<CapituloCriado, ErroDeComando> {
    estado.sessao().criar_capitulo(&nome)
}

/// A pasta de dados do app, onde fica a lista dos livros.
fn pasta_de_dados(app: &AppHandle) -> Result<PathBuf, ErroDeComando> {
    app.path().app_data_dir().map_err(|erro| ErroDeComando {
        codigo: "erro",
        mensagem: format!("não achei a pasta de dados do md Novel: {erro}"),
    })
}

/// O caminho que a página mandou, se ele está na lista dos livros lembrados.
fn livro_lembrado(app: &AppHandle, pasta: &str) -> Result<PathBuf, ErroDeComando> {
    memoria::lembrado(&pasta_de_dados(app)?, pasta).map_err(|mensagem| ErroDeComando {
        codigo: "erro",
        mensagem,
    })
}

/// Os livros já abertos, para o menu do nome do livro (ADR 0003, "Vários
/// livros"). Livro cuja pasta sumiu fica de fora.
#[tauri::command(async)]
fn livros_lembrados(app: AppHandle, estado: State<'_, Estado>) -> Vec<LivroLembrado> {
    let aberto = estado.sessao().pasta().map(Path::to_owned);
    let livros = pasta_de_dados(&app)
        .map(|dados| memoria::ler_livros(&dados))
        .unwrap_or_default();
    memoria::para_o_menu(&livros, aberto.as_deref())
}

/// Abre um livro do menu. A página devolve o caminho que recebeu na lista, e
/// só um que está nela vale: caminho novo vem só do seletor do sistema.
#[tauri::command(async)]
fn abrir_livro_lembrado(
    pasta: String,
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<LivroAberto, ErroDeComando> {
    let caminho = livro_lembrado(&app, &pasta)?;
    let livro = estado.sessao().abrir_pasta(&caminho)?;
    lembrar(&app, Path::new(&livro.pasta));
    Ok(livro)
}

/// Tira um livro da lista, sem apagar nada do disco, e devolve a lista nova.
/// O livro aberto não sai: o menu sempre mostra onde o autor está.
#[tauri::command(async)]
fn esquecer_livro(
    pasta: String,
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<Vec<LivroLembrado>, ErroDeComando> {
    let aberto = estado.sessao().pasta().map(Path::to_owned);
    let dados = pasta_de_dados(&app)?;
    memoria::tirar_da_lista(&dados, &pasta, aberto.as_deref()).map_err(|mensagem| {
        ErroDeComando {
            codigo: "erro",
            mensagem,
        }
    })?;
    Ok(memoria::para_o_menu(
        &memoria::ler_livros(&dados),
        aberto.as_deref(),
    ))
}

/// Renomeia a pasta do livro aberto. A lista de livros troca o caminho no
/// mesmo lugar; o que a página lembra, ela troca com o `antes` devolvido.
#[tauri::command(async)]
fn renomear_livro(
    nome: String,
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<LivroRenomeado, ErroDeComando> {
    let renomeado = estado.sessao().renomear_livro(&nome)?;
    if let Ok(dados) = app.path().app_data_dir() {
        let trocado = memoria::trocar(
            &dados,
            Path::new(&renomeado.antes),
            Path::new(&renomeado.livro.pasta),
        );
        if let Err(erro) = trocado {
            eprintln!("aviso: não consegui guardar o nome novo do livro na lista: {erro}");
        }
    }
    Ok(renomeado)
}

/// Renomeia o arquivo de um capítulo da lista. A página manda o nome atual,
/// que vem da lista, e o novo, que o autor digitou e o `workspace` confere.
#[tauri::command(async)]
fn renomear_capitulo(
    atual: String,
    novo: String,
    estado: State<'_, Estado>,
) -> Result<CapituloCriado, ErroDeComando> {
    estado.sessao().renomear_capitulo(&atual, &novo)
}

/// Procura um termo em todos os capítulos do livro aberto (ADR 0003,
/// "Buscar no livro"). A página manda só o termo; o Rust lê os capítulos do
/// disco e nada grava. A sessão só fica travada para dizer qual é o livro:
/// a leitura roda com ela livre, e quem quer salvar não espera a busca.
#[tauri::command(async)]
fn buscar_no_livro(
    termo: String,
    estado: State<'_, Estado>,
) -> Result<BuscaNoLivro, ErroDeComando> {
    let livro = estado.sessao().livro_para_buscar()?;
    Sessao::buscar(&livro, &termo)
}

/// Cria a pasta de um livro novo. O nome vem do autor e é conferido antes de
/// abrir o seletor; onde ela fica, o autor escolhe no seletor do sistema,
/// que abre na pasta de cima do livro aberto. `None` se ele cancelou.
#[tauri::command(async)]
fn criar_livro(
    nome: String,
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<Option<LivroAberto>, ErroDeComando> {
    let nome = workspace::nome_de_livro(&nome)?;
    let onde = match onde_criar_de_teste() {
        Some(onde) => onde,
        None => {
            let de_cima = estado
                .sessao()
                .pasta()
                .and_then(Path::parent)
                .map(Path::to_owned);
            let mut seletor = app
                .dialog()
                .file()
                .set_title(format!("Onde criar a pasta do livro \"{nome}\""));
            if let Some(de_cima) = de_cima {
                seletor = seletor.set_directory(de_cima);
            }
            let Some(escolhida) = seletor.blocking_pick_folder() else {
                return Ok(None);
            };
            escolhida.into_path().map_err(|erro| ErroDeComando {
                codigo: "erro",
                mensagem: format!("a pasta escolhida não é um caminho deste computador: {erro}"),
            })?
        }
    };
    let livro = estado.sessao().criar_livro(&onde, &nome)?;
    lembrar(&app, Path::new(&livro.pasta));
    Ok(Some(livro))
}

/// Onde criar o livro no teste funcional, no lugar do seletor do sistema. Só
/// existe em build de depuração.
fn onde_criar_de_teste() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        std::env::var_os("MD_NOVEL_TESTE_ONDE_CRIAR_LIVRO").map(PathBuf::from)
    } else {
        None
    }
}

/// Carrega as faces de uma fonte instalada pelo usuário no macOS (ADR 0003).
/// A página manda só o nome da família, nunca um caminho; o Rust lê apenas das
/// pastas padrão de fontes. Fora do macOS o WebView já vê as fontes do usuário.
#[tauri::command(async)]
fn carregar_fonte_usuario(familia: String) -> Vec<FaceDeFonte> {
    #[cfg(target_os = "macos")]
    {
        fontes::carregar_do_mac(&familia)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = familia;
        Vec::new()
    }
}

/// Entra na tela cheia ou sai dela. Devolve se a janela ficou em tela cheia.
#[tauri::command]
fn alternar_tela_cheia(janela: Window) -> Result<bool, ErroDeComando> {
    let falhou = |erro: tauri::Error| ErroDeComando {
        codigo: "erro",
        mensagem: format!("não consegui mudar a tela cheia: {erro}"),
    };
    let cheia = !janela.is_fullscreen().map_err(falhou)?;
    janela.set_fullscreen(cheia).map_err(falhou)?;
    Ok(cheia)
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
    let livro = estado.sessao().abrir_pasta(&pasta)?;
    lembrar(&app, Path::new(&livro.pasta));
    Ok(Some(livro))
}

/// A língua pedida ao corretor. Em build de depuração o teste funcional pode
/// pedir outra, para conferir o aviso de dicionário ausente.
#[cfg(target_os = "linux")]
fn lingua_do_corretor() -> String {
    if cfg!(debug_assertions) {
        if let Ok(lingua) = std::env::var("MD_NOVEL_TESTE_LINGUA_DO_CORRETOR") {
            return lingua;
        }
    }
    corretor::LINGUA.to_owned()
}

/// Liga a correção ortográfica do WebKitGTK na língua do app e guarda o que
/// ele carregou. Sem isto o `spellcheck` da página não faz nada no Linux.
#[cfg(target_os = "linux")]
fn ligar_o_corretor(app: &AppHandle) {
    use webkit2gtk::{WebContextExt, WebViewExt};

    let Some(janela) = app.get_webview_window("main") else {
        return;
    };
    let lingua = lingua_do_corretor();
    let para_guardar = app.clone();
    let ligado = janela.with_webview(move |webview| {
        let Some(contexto) = webview.inner().context() else {
            return;
        };
        contexto.set_spell_checking_languages(&[lingua.as_str()]);
        contexto.set_spell_checking_enabled(true);
        // O WebKitGTK só fica com a língua que tem dicionário instalado.
        let linguas: Vec<String> = contexto
            .spell_checking_languages()
            .iter()
            .map(ToString::to_string)
            .collect();
        let aviso = corretor::aviso_sem_dicionario(&lingua, &linguas);
        *para_guardar
            .state::<Estado>()
            .corretor
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = EstadoDoCorretor { linguas, aviso };
    });
    if let Err(erro) = ligado {
        eprintln!("aviso: não consegui ligar o corretor ortográfico: {erro}");
    }
}

/// No Mac o WebView do sistema já corrige, com o dicionário do sistema.
#[cfg(not(target_os = "linux"))]
fn ligar_o_corretor(_app: &AppHandle) {}

/// O que o corretor carregou, para a página avisar se falta o dicionário.
#[tauri::command]
fn estado_do_corretor(estado: State<'_, Estado>) -> EstadoDoCorretor {
    estado
        .corretor
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Põe o livro aberto no alto da lista dos lembrados, para o menu e para a
/// próxima abertura do app.
/// Não conseguir guardar não atrapalha o trabalho: só fica o aviso no terminal.
fn lembrar(app: &AppHandle, pasta: &Path) {
    let Ok(dados) = app.path().app_data_dir() else {
        return;
    };
    if let Err(erro) = memoria::lembrar(&dados, pasta) {
        eprintln!("aviso: não consegui guardar o último livro aberto: {erro}");
    }
}

/// Ao abrir o app: o livro da linha de comando (`md-novel-app <pasta>`) ou,
/// sem ela, o último que o autor abriu. Se o último sumiu (pasta apagada,
/// disco fora), o app abre sem livro, como na primeira vez.
fn abrir_o_livro_inicial(app: &AppHandle) {
    let pedida = std::env::args_os().nth(1).map(PathBuf::from);
    let lembrada = || {
        let dados = app.path().app_data_dir().ok()?;
        memoria::ler_ultimo_livro(&dados)
    };
    let foi_pedida = pedida.is_some();
    let Some(pasta) = pedida.or_else(lembrada) else {
        return;
    };
    let aberto = app.state::<Estado>().sessao().abrir_pasta(&pasta);
    match aberto {
        Ok(livro) => lembrar(app, Path::new(&livro.pasta)),
        Err(erro) => {
            if foi_pedida {
                eprintln!("erro: {}", erro.mensagem);
            }
        }
    }
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
    /// Criar o estilo.yaml pelo botão do aviso, depois de digitar.
    criar_estilo: bool,
    /// Usar o zoom, esconder a lateral e ligar a máquina de escrever.
    conforto: bool,
    /// Digitar aspas e hifens e abrir a busca.
    escrita: bool,
    /// Definir a meta do dia, trocar de capítulo e mover o cursor.
    sessao: bool,
    /// Criar um capítulo pelo botão da lateral.
    capitulo_novo: bool,
    /// Digitar palavras erradas, com pausas para a captura da tela.
    corretor: bool,
    /// Mudar o estilo pelo painel.
    painel_de_estilo: bool,
    /// Medir a linha em branco escondida, criar e juntar parágrafo, converter sinais.
    paragrafos: bool,
    /// Renomear capítulo e livro, trocar de livro, tirar da lista e criar livro.
    livros: bool,
    /// Buscar no livro inteiro e abrir um achado de outro capítulo.
    busca: bool,
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
        criar_estilo: std::env::var_os("MD_NOVEL_TESTE_CRIAR_ESTILO").is_some(),
        conforto: std::env::var_os("MD_NOVEL_TESTE_CONFORTO").is_some(),
        escrita: std::env::var_os("MD_NOVEL_TESTE_ESCRITA").is_some(),
        sessao: std::env::var_os("MD_NOVEL_TESTE_SESSAO").is_some(),
        capitulo_novo: std::env::var_os("MD_NOVEL_TESTE_CAPITULO_NOVO").is_some(),
        corretor: std::env::var_os("MD_NOVEL_TESTE_CORRETOR").is_some(),
        painel_de_estilo: std::env::var_os("MD_NOVEL_TESTE_PAINEL_DE_ESTILO").is_some(),
        paragrafos: std::env::var_os("MD_NOVEL_TESTE_PARAGRAFOS").is_some(),
        livros: std::env::var_os("MD_NOVEL_TESTE_LIVROS").is_some(),
        busca: std::env::var_os("MD_NOVEL_TESTE_BUSCA").is_some(),
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
    let resultado = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Estado::default())
        // Antes de a página carregar: quando ela pedir o livro, ele já está aberto.
        .setup(|app| {
            abrir_o_livro_inicial(app.handle());
            ligar_o_corretor(app.handle());
            Ok(())
        })
        .on_window_event(ao_pedir_para_fechar)
        .invoke_handler(tauri::generate_handler![
            livro_atual,
            escolher_pasta,
            criar_estilo,
            estilo_para_editar,
            salvar_estilo,
            criar_capitulo,
            livros_lembrados,
            abrir_livro_lembrado,
            esquecer_livro,
            renomear_livro,
            renomear_capitulo,
            criar_livro,
            buscar_no_livro,
            carregar_fonte_usuario,
            estado_do_corretor,
            alternar_tela_cheia,
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
        eprintln!("erro: a janela do md Novel não abriu: {erro}");
        std::process::exit(1);
    }
}
