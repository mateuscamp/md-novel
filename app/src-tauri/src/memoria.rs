//! O que o app lembra entre uma abertura e outra: o último livro aberto.
//! Fica na pasta de dados do app, nunca na pasta do livro. Sem tauri: quem
//! diz onde é a pasta de dados é o `main.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use workspace::{ErroWorkspace, SaidaPendente};

const ARQUIVO: &str = "ultimo-livro.txt";

/// Guarda `pasta` como o último livro aberto. A gravação passa pelo
/// `workspace`, como toda gravação do projeto.
pub fn guardar_ultimo_livro(dados: &Path, pasta: &Path) -> Result<(), ErroWorkspace> {
    // Um caminho fora do UTF-8 não cabe num arquivo de texto: o app
    // simplesmente não lembra dele.
    let Some(texto) = pasta.to_str() else {
        return Ok(());
    };
    fs::create_dir_all(dados)?;
    let mut saida = SaidaPendente::preparar(&dados.join(ARQUIVO), "txt")?;
    saida.escrever(texto.as_bytes())?;
    saida.publicar()
}

/// O último livro aberto, se o registro existe e a pasta ainda está lá. Um
/// registro estragado é o mesmo que não ter registro: o app abre sem livro.
pub fn ler_ultimo_livro(dados: &Path) -> Option<PathBuf> {
    // Sem teto de tamanho: o arquivo é do próprio app, e um caminho maior do
    // que o sistema aceita já é recusado abaixo, por não ser pasta.
    let pasta = PathBuf::from(fs::read_to_string(dados.join(ARQUIVO)).ok()?);
    (pasta.is_absolute() && pasta.is_dir()).then_some(pasta)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn o_ultimo_livro_guardado_volta_e_o_mais_novo_troca_o_antigo() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let (um, dois) = (
            tempfile::tempdir().expect("livro"),
            tempfile::tempdir().expect("livro"),
        );
        assert_eq!(ler_ultimo_livro(dados.path()), None);

        guardar_ultimo_livro(dados.path(), um.path()).expect("guardar");
        assert_eq!(ler_ultimo_livro(dados.path()), Some(um.path().to_owned()));
        guardar_ultimo_livro(dados.path(), dois.path()).expect("guardar de novo");
        assert_eq!(ler_ultimo_livro(dados.path()), Some(dois.path().to_owned()));

        // Um arquivo só, sem temporário esquecido.
        let nomes: Vec<_> = fs::read_dir(dados.path())
            .expect("listar")
            .map(|entrada| entrada.expect("entrada").file_name())
            .collect();
        assert_eq!(nomes, ["ultimo-livro.txt"]);
    }

    #[test]
    fn a_pasta_de_dados_e_criada_se_ainda_nao_existe() {
        let base = tempfile::tempdir().expect("base");
        let dados = base.path().join("ainda/nao/existe");
        let livro = tempfile::tempdir().expect("livro");
        guardar_ultimo_livro(&dados, livro.path()).expect("guardar");
        assert_eq!(ler_ultimo_livro(&dados), Some(livro.path().to_owned()));
    }

    #[test]
    fn livro_que_sumiu_ou_registro_estragado_nao_e_reaberto() {
        let dados = tempfile::tempdir().expect("pasta de dados");
        let livro = tempfile::tempdir().expect("livro");
        let caminho = livro.path().to_owned();
        guardar_ultimo_livro(dados.path(), &caminho).expect("guardar");
        drop(livro);
        assert_eq!(ler_ultimo_livro(dados.path()), None, "a pasta foi apagada");

        let registro = dados.path().join("ultimo-livro.txt");
        let existente = tempfile::tempdir().expect("livro");
        // Caminho relativo, arquivo no lugar de pasta, bytes fora do UTF-8,
        // vazio e mais comprido do que um caminho pode ser: nada disso abre
        // livro nenhum.
        fs::write(&registro, "relativo/livro").expect("escrever");
        assert_eq!(ler_ultimo_livro(dados.path()), None);
        fs::write(&registro, registro.to_str().expect("utf-8")).expect("escrever");
        assert_eq!(ler_ultimo_livro(dados.path()), None);
        fs::write(&registro, [0xff, 0xfe]).expect("escrever");
        assert_eq!(ler_ultimo_livro(dados.path()), None);
        fs::write(&registro, "").expect("escrever");
        assert_eq!(ler_ultimo_livro(dados.path()), None);
        let enorme = format!(
            "{}{}",
            existente.path().to_str().expect("utf-8"),
            "/".repeat(70 * 1024)
        );
        fs::write(&registro, enorme).expect("escrever");
        assert_eq!(ler_ultimo_livro(dados.path()), None);
    }
}
