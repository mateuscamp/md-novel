//! O que o app lembra entre uma abertura e outra: os livros já abertos, o
//! mais recente primeiro. Fica na pasta de dados do app, nunca na pasta do
//! livro. Sem tauri: quem diz onde é a pasta de dados é o `main.rs`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;
use workspace::{ErroWorkspace, SaidaPendente};

/// Um caminho por linha, o livro aberto por último na primeira.
const ARQUIVO: &str = "livros.txt";
/// O registro de antes, só com o último livro. Lido quando o novo ainda não
/// existe, e nunca apagado: o app só apaga temporário dele mesmo.
const ARQUIVO_ANTIGO: &str = "ultimo-livro.txt";
/// Mais do que isso não cabe numa lista de olhar e escolher. Abrir o 51º
/// esquece o mais antigo; a pasta dele continua no disco.
pub const MAXIMO_DE_LIVROS: usize = 50;

/// Os livros lembrados, o mais recente primeiro: só caminhos inteiros de
/// pastas que ainda existem, sem repetição. Um registro estragado é o mesmo
/// que não ter registro; uma linha estragada fica de fora e o resto vale.
pub fn ler_livros(dados: &Path) -> Vec<PathBuf> {
    ler_com(dados, |_| false)
}

/// A lista como `ler_livros`, mas guardando também os caminhos que `manter`
/// aceita mesmo sem a pasta existir (a de um livro que acabou de mudar de nome).
fn ler_com(dados: &Path, manter: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    // Sem teto de tamanho: o arquivo é do próprio app, e um caminho maior do
    // que o sistema aceita já fica de fora abaixo, por não ser pasta.
    let bytes = match fs::read(dados.join(ARQUIVO)) {
        Ok(bytes) => bytes,
        Err(erro) if erro.kind() == ErrorKind::NotFound => {
            fs::read(dados.join(ARQUIVO_ANTIGO)).unwrap_or_default()
        }
        Err(_) => Vec::new(),
    };
    let Ok(texto) = String::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut livros: Vec<PathBuf> = Vec::new();
    for linha in texto.lines() {
        let pasta = PathBuf::from(linha);
        let vale = pasta.is_absolute() && (pasta.is_dir() || manter(&pasta));
        if vale && !livros.contains(&pasta) {
            livros.push(pasta);
        }
    }
    livros.truncate(MAXIMO_DE_LIVROS);
    livros
}

/// O livro aberto por último, para a próxima abertura do app.
pub fn ler_ultimo_livro(dados: &Path) -> Option<PathBuf> {
    ler_livros(dados).into_iter().next()
}

/// Põe `pasta` no alto da lista, como o livro aberto agora.
pub fn lembrar(dados: &Path, pasta: &Path) -> Result<(), ErroWorkspace> {
    let mut livros = ler_livros(dados);
    livros.retain(|livro| livro != pasta);
    livros.insert(0, pasta.to_owned());
    guardar(dados, &livros)
}

/// O livro renomeado continua no mesmo lugar da lista, com o caminho novo.
/// Se o antigo não estava na lista, o novo entra no alto.
pub fn trocar(dados: &Path, antiga: &Path, nova: &Path) -> Result<(), ErroWorkspace> {
    // O sistema já renomeou: a pasta antiga não existe mais, e a leitura
    // comum a deixaria de fora, junto com o lugar dela.
    let mut livros = ler_com(dados, |pasta| pasta == antiga);
    livros.retain(|livro| livro != nova);
    match livros.iter().position(|livro| livro == antiga) {
        Some(lugar) => livros[lugar] = nova.to_owned(),
        None => livros.insert(0, nova.to_owned()),
    }
    guardar(dados, &livros)
}

/// Tira o livro da lista. A pasta continua no disco.
pub fn esquecer(dados: &Path, pasta: &Path) -> Result<(), ErroWorkspace> {
    let mut livros = ler_livros(dados);
    livros.retain(|livro| livro != pasta);
    guardar(dados, &livros)
}

/// O caminho que a página mandou, se ele é de um livro da lista. A página
/// só manda o que o Rust lhe deu (ADR 0003): outro caminho é recusado.
pub fn lembrado(dados: &Path, pasta: &str) -> Result<PathBuf, String> {
    let pedido = Path::new(pasta);
    ler_livros(dados)
        .into_iter()
        .find(|livro| livro.as_os_str() == pedido.as_os_str())
        .ok_or_else(|| {
            format!(
                "o livro {pasta:?} não está na lista dos livros abertos no md Novel; \
                 abra a pasta dele por \"Abrir outra pasta…\""
            )
        })
}

/// Um livro no menu do nome do livro. A página mostra o nome e a pasta de
/// cima, e devolve `pasta` (o caminho que o Rust deu) para abrir ou tirar.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LivroLembrado {
    pub pasta: String,
    pub nome: String,
    /// O nome da pasta onde o livro está, para separar dois com o mesmo nome.
    pub onde: String,
    pub aberto: bool,
}

/// A lista do menu, na ordem lembrada. Caminho fora do UTF-8 não tem como
/// ir e voltar pela página: fica de fora.
pub fn para_o_menu(livros: &[PathBuf], aberto: Option<&Path>) -> Vec<LivroLembrado> {
    let nome = |pasta: &Path| {
        pasta.file_name().map_or_else(
            || pasta.display().to_string(),
            |nome| nome.to_string_lossy().into_owned(),
        )
    };
    livros
        .iter()
        .filter_map(|pasta| {
            Some(LivroLembrado {
                pasta: pasta.to_str()?.to_owned(),
                nome: nome(pasta),
                onde: pasta.parent().map(nome).unwrap_or_default(),
                aberto: aberto == Some(pasta.as_path()),
            })
        })
        .collect()
}

/// O pedido da página de tirar um livro da lista. Só um livro que está nela,
/// e nunca o aberto: o menu sempre mostra onde o autor está. A pasta fica.
pub fn tirar_da_lista(dados: &Path, pasta: &str, aberto: Option<&Path>) -> Result<(), String> {
    let caminho = lembrado(dados, pasta)?;
    if aberto == Some(caminho.as_path()) {
        return Err(format!(
            "o livro {pasta:?} está aberto e não sai da lista; abra outro antes de tirá-lo"
        ));
    }
    esquecer(dados, &caminho).map_err(|erro| erro.to_string())
}

/// Grava a lista inteira pela `SaidaPendente`, como toda gravação do
/// projeto. Caminho fora do UTF-8 ou com quebra de linha não cabe numa linha
/// de texto: o app simplesmente não lembra dele.
fn guardar(dados: &Path, livros: &[PathBuf]) -> Result<(), ErroWorkspace> {
    let linhas: Vec<&str> = livros
        .iter()
        .filter_map(|livro| livro.to_str())
        .filter(|linha| !linha.contains(['\n', '\r']))
        .take(MAXIMO_DE_LIVROS)
        .collect();
    fs::create_dir_all(dados)?;
    let mut saida = SaidaPendente::preparar(&dados.join(ARQUIVO), "txt")?;
    saida.escrever(linhas.join("\n").as_bytes())?;
    saida.publicar()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn nomes(pasta: &Path) -> Vec<String> {
        let mut nomes: Vec<String> = fs::read_dir(pasta)
            .expect("listar")
            .map(|entrada| {
                entrada
                    .expect("entrada")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        nomes.sort();
        nomes
    }

    fn livros(quantos: usize) -> Vec<tempfile::TempDir> {
        (0..quantos)
            .map(|_| tempfile::tempdir().expect("livro"))
            .collect()
    }

    #[test]
    fn o_ultimo_livro_aberto_vem_primeiro_e_nenhum_se_repete() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let [um, dois] = [livros(1).remove(0), livros(1).remove(0)];
        assert_eq!(ler_livros(dados.path()), Vec::<PathBuf>::new());
        assert_eq!(ler_ultimo_livro(dados.path()), None);

        lembrar(dados.path(), um.path()).expect("lembrar");
        lembrar(dados.path(), dois.path()).expect("lembrar");
        assert_eq!(ler_livros(dados.path()), [dois.path(), um.path()]);
        lembrar(dados.path(), um.path()).expect("lembrar de novo");
        assert_eq!(ler_livros(dados.path()), [um.path(), dois.path()]);
        assert_eq!(ler_ultimo_livro(dados.path()), Some(um.path().to_owned()));

        // Um arquivo só, sem temporário esquecido.
        assert_eq!(nomes(dados.path()), ["livros.txt"]);
    }

    #[test]
    fn a_lista_para_em_cinquenta_livros_e_esquece_o_mais_antigo() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let todos = livros(MAXIMO_DE_LIVROS + 2);
        for livro in &todos {
            lembrar(dados.path(), livro.path()).expect("lembrar");
        }
        let lidos = ler_livros(dados.path());
        assert_eq!(lidos.len(), MAXIMO_DE_LIVROS);
        assert_eq!(
            lidos.first().map(PathBuf::as_path),
            todos.last().map(|livro| livro.path())
        );
        assert_eq!(lidos.last().map(PathBuf::as_path), Some(todos[2].path()));
    }

    #[test]
    fn o_registro_antigo_de_um_livro_so_ainda_vale_e_nao_e_apagado() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let [antigo, novo] = [livros(1).remove(0), livros(1).remove(0)];
        fs::write(
            dados.path().join("ultimo-livro.txt"),
            antigo.path().to_str().expect("utf-8"),
        )
        .expect("escrever o registro antigo");
        assert_eq!(ler_livros(dados.path()), [antigo.path()]);

        lembrar(dados.path(), novo.path()).expect("lembrar");
        assert_eq!(ler_livros(dados.path()), [novo.path(), antigo.path()]);
        assert_eq!(nomes(dados.path()), ["livros.txt", "ultimo-livro.txt"]);
    }

    #[test]
    fn renomear_troca_o_caminho_no_mesmo_lugar_e_esquecer_tira_so_aquele() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let base = tempfile::tempdir().expect("base");
        let [um, dois, tres, renomeado] = ["um", "dois", "tres", "Dois renomeado"].map(|nome| {
            let pasta = base.path().join(nome);
            fs::create_dir(&pasta).expect("criar livro");
            pasta
        });
        for pasta in [&um, &dois, &tres] {
            lembrar(dados.path(), pasta).expect("lembrar");
        }
        assert_eq!(
            ler_livros(dados.path()),
            [tres.clone(), dois.clone(), um.clone()]
        );

        // O sistema já renomeou a pasta: a velha não existe mais.
        fs::remove_dir(&dois).expect("a pasta velha some");
        trocar(dados.path(), &dois, &renomeado).expect("trocar");
        assert_eq!(
            ler_livros(dados.path()),
            [tres.clone(), renomeado.clone(), um.clone()]
        );

        esquecer(dados.path(), &tres).expect("esquecer");
        assert_eq!(ler_livros(dados.path()), [renomeado.clone(), um.clone()]);
        assert!(tres.is_dir(), "esquecer não apaga a pasta");
    }

    #[test]
    fn a_pagina_so_abre_um_livro_que_esta_na_lista() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let [lembrado_, fora] = [livros(1).remove(0), livros(1).remove(0)];
        lembrar(dados.path(), lembrado_.path()).expect("lembrar");
        let texto = lembrado_.path().to_str().expect("utf-8");
        assert_eq!(
            lembrado(dados.path(), texto),
            Ok(lembrado_.path().to_owned())
        );

        let de_fora = fora.path().to_str().expect("utf-8");
        let relativo = format!(
            "{texto}/../{}",
            fora.path().file_name().expect("nome").to_string_lossy()
        );
        for recusado in [de_fora, "relativo/livro", "", "/", relativo.as_str()] {
            let erro = lembrado(dados.path(), recusado).expect_err(recusado);
            assert!(
                erro.contains(&format!("{recusado:?}")),
                "sem o valor recebido: {erro}"
            );
        }
    }

    #[test]
    fn a_pasta_de_dados_e_criada_se_ainda_nao_existe() {
        let base = tempfile::tempdir().expect("base");
        let dados = base.path().join("ainda/nao/existe");
        let livro = tempfile::tempdir().expect("livro");
        lembrar(&dados, livro.path()).expect("lembrar");
        assert_eq!(ler_ultimo_livro(&dados), Some(livro.path().to_owned()));
    }

    #[test]
    fn caminho_com_quebra_de_linha_nao_e_lembrado() {
        // Ele viraria dois caminhos na leitura: o app simplesmente não lembra.
        let dados = tempfile::tempdir().expect("pasta de dados");
        let base = tempfile::tempdir().expect("base");
        let quebrado = base.path().join("um\ndois");
        fs::create_dir(&quebrado).expect("criar");
        fs::create_dir(base.path().join("um")).expect("criar");
        lembrar(dados.path(), &quebrado).expect("lembrar");
        assert_eq!(ler_livros(dados.path()), Vec::<PathBuf>::new());
    }

    #[test]
    fn livro_que_sumiu_ou_linha_estragada_fica_de_fora_e_o_resto_vale() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let [bom, sumiu] = [livros(1).remove(0), livros(1).remove(0)];
        let caminho_que_sumiu = sumiu.path().to_owned();
        drop(sumiu);
        let registro = dados.path().join("livros.txt");
        let bom_texto = bom.path().to_str().expect("utf-8");
        let enorme = format!("{bom_texto}{}", "/".repeat(70 * 1024));
        // Relativo, arquivo no lugar de pasta, vazio, mais comprido do que
        // um caminho pode ser e pasta apagada: nada disso entra; o bom, sim.
        let linhas = [
            "relativo/livro",
            registro.to_str().expect("utf-8"),
            "",
            enorme.as_str(),
            caminho_que_sumiu.to_str().expect("utf-8"),
            bom_texto,
            bom_texto,
        ];
        fs::write(&registro, linhas.join("\n")).expect("escrever");
        assert_eq!(ler_livros(dados.path()), [bom.path()]);

        // Bytes fora do UTF-8: o registro inteiro não vale.
        fs::write(&registro, [0xff, 0xfe]).expect("escrever");
        assert_eq!(ler_livros(dados.path()), Vec::<PathBuf>::new());
    }

    #[test]
    fn o_menu_mostra_nome_e_pasta_de_cima_e_marca_o_aberto() {
        let livros = [
            PathBuf::from("/home/autor/Textos/O dom da pele"),
            PathBuf::from("/home/autor/Rascunhos/md"),
        ];
        let menu = para_o_menu(&livros, Some(Path::new("/home/autor/Rascunhos/md")));
        assert_eq!(
            menu,
            [
                LivroLembrado {
                    pasta: "/home/autor/Textos/O dom da pele".to_owned(),
                    nome: "O dom da pele".to_owned(),
                    onde: "Textos".to_owned(),
                    aberto: false,
                },
                LivroLembrado {
                    pasta: "/home/autor/Rascunhos/md".to_owned(),
                    nome: "md".to_owned(),
                    onde: "Rascunhos".to_owned(),
                    aberto: true,
                },
            ]
        );
        assert!(para_o_menu(&livros, None).iter().all(|livro| !livro.aberto));
    }

    #[test]
    fn o_livro_aberto_nao_sai_da_lista_e_os_outros_saem_sem_apagar_nada() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let [aberto, outro] = [livros(1).remove(0), livros(1).remove(0)];
        lembrar(dados.path(), outro.path()).expect("lembrar");
        lembrar(dados.path(), aberto.path()).expect("lembrar");
        let texto = |livro: &tempfile::TempDir| livro.path().to_str().expect("utf-8").to_owned();

        let erro = tirar_da_lista(dados.path(), &texto(&aberto), Some(aberto.path()))
            .expect_err("o aberto fica");
        assert!(
            erro.contains(&format!("{:?}", texto(&aberto))),
            "sem o valor recebido: {erro}"
        );
        assert_eq!(ler_livros(dados.path()), [aberto.path(), outro.path()]);

        tirar_da_lista(dados.path(), &texto(&outro), Some(aberto.path())).expect("tirar o outro");
        assert_eq!(ler_livros(dados.path()), [aberto.path()]);
        assert!(outro.path().is_dir(), "a pasta continua no disco");
        // O que já saiu, ou nunca esteve, não está na lista.
        assert!(tirar_da_lista(dados.path(), &texto(&outro), None).is_err());
    }
}
