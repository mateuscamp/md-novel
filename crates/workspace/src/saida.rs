//! Publicação de arquivo gerado: escreve num temporário ao lado do destino e
//! troca com rename, para quem lê ou sincroniza a pasta (Dropbox) nunca ver o
//! arquivo pela metade.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use std::time::Duration;

use tempfile::NamedTempFile;

use crate::ErroWorkspace;

const PREFIXO_DO_TEMPORARIO: &str = ".md-novel-gravando-";
// `.md` entrou com o app: salvar um capítulo também passa por um temporário.
const EXTENSOES_GERADAS: [&str; 3] = [".css", ".docx", ".md"];
const IDADE_DE_ORFAO: Duration = Duration::from_secs(60 * 60);

/// Uma saída já escrita (ou a escrever) num temporário, ainda não publicada.
/// Se for descartada sem `publicar`, o temporário some e o destino fica intacto.
///
/// ```no_run
/// use std::path::Path;
/// let mut css = workspace::SaidaPendente::preparar(Path::new("editor.css"), "css")?;
/// css.escrever(b"p { margin: 0 }")?;
/// css.publicar()?;
/// # Ok::<(), workspace::ErroWorkspace>(())
/// ```
#[derive(Debug)]
pub struct SaidaPendente {
    temporario: NamedTempFile,
    destino: PathBuf,
}

impl SaidaPendente {
    /// Confere o destino e cria o temporário na pasta dele.
    ///
    /// A extensão é exigida porque o manuscrito é o ativo: um argumento
    /// trocado não pode gravar por cima de um capítulo ou do estilo.yaml.
    pub fn preparar(destino: &Path, extensao: &str) -> Result<Self, ErroWorkspace> {
        let invalida = || ErroWorkspace::SaidaInvalida {
            recebido: destino.to_owned(),
        };
        let extensao_errada = |caminho: &Path| ErroWorkspace::ExtensaoErrada {
            recebido: caminho.to_owned(),
            esperada: extensao.to_owned(),
        };
        if destino.file_name().is_none() {
            return Err(invalida());
        }
        if !tem_extensao(destino, extensao) {
            return Err(extensao_errada(destino));
        }
        // O rename trocaria um link simbólico por um arquivo comum e deixaria
        // o arquivo real com o conteúdo antigo. A saída vai para o alvo do
        // link, que passa pela mesma guarda de extensão: um `editor.css` que
        // aponta para um capítulo não pode servir de porta para sobrescrevê-lo.
        let destino = &resolver_link(destino).ok_or_else(invalida)?;
        if !tem_extensao(destino, extensao) {
            return Err(extensao_errada(destino));
        }
        let pasta = match destino.parent() {
            Some(pasta) if !pasta.as_os_str().is_empty() => pasta,
            Some(_) | None => Path::new("."),
        };
        if !pasta.is_dir() {
            return Err(ErroWorkspace::PastaInexistente {
                recebido: pasta.to_owned(),
            });
        }
        if destino.is_dir() {
            return Err(invalida());
        }
        recolher_orfaos(pasta);
        // `tempfile` cria com nome imprevisível e permissão 0600.
        let temporario = tempfile::Builder::new()
            .prefix(PREFIXO_DO_TEMPORARIO)
            .suffix(&format!(".{extensao}"))
            .tempfile_in(pasta)?;
        Ok(SaidaPendente {
            temporario,
            destino: destino.to_owned(),
        })
    }

    /// Onde um programa externo (o Pandoc) deve escrever o conteúdo.
    pub fn caminho_temporario(&self) -> &Path {
        self.temporario.path()
    }

    pub fn escrever(&mut self, conteudo: &[u8]) -> Result<(), ErroWorkspace> {
        self.temporario.write_all(conteudo)?;
        Ok(())
    }

    /// Troca o destino pelo temporário. Um destino que já existia mantém a
    /// permissão que tinha; um novo fica com a do temporário (0600).
    pub fn publicar(self) -> Result<(), ErroWorkspace> {
        // Pelo caminho, não pelo descritor: o Pandoc pode ter trocado o
        // arquivo temporário por outro com o mesmo nome.
        if let Ok(antigo) = fs::metadata(&self.destino) {
            fs::set_permissions(self.temporario.path(), antigo.permissions())?;
        }
        File::open(self.temporario.path())?.sync_all()?;
        self.temporario
            .persist(&self.destino)
            .map_err(|erro| erro.error)?;
        Ok(())
    }
}

/// O alvo real de um destino que é link simbólico; o próprio destino nos
/// outros casos. `None` quando o link está quebrado.
fn resolver_link(destino: &Path) -> Option<PathBuf> {
    match fs::symlink_metadata(destino) {
        Ok(metadados) if metadados.file_type().is_symlink() => fs::canonicalize(destino).ok(),
        Ok(_) | Err(_) => Some(destino.to_owned()),
    }
}

/// Apaga temporário do md-novel que uma execução interrompida (Ctrl-C, queda
/// de energia) deixou na pasta. Só arquivo comum, com o prefixo e a extensão
/// exatos que este módulo cria, parado há mais de uma hora: nada do autor tem
/// esse nome, e uma exportação em andamento em outra janela é mais nova.
/// É faxina de melhor esforço: se não der para ler ou apagar, a saída segue.
fn recolher_orfaos(pasta: &Path) {
    let Ok(entradas) = fs::read_dir(pasta) else {
        return;
    };
    for entrada in entradas.flatten() {
        let nome = entrada.file_name();
        let e_do_md_novel = nome.to_str().is_some_and(|nome| {
            nome.starts_with(PREFIXO_DO_TEMPORARIO)
                && EXTENSOES_GERADAS
                    .iter()
                    .any(|extensao| nome.ends_with(extensao))
        });
        // O nome decide primeiro: assim só o que parece temporário custa uma
        // consulta de metadados, e não cada arquivo da pasta do livro.
        if !e_do_md_novel {
            continue;
        }
        // `DirEntry::metadata` não segue link: um link com esse nome fica.
        let parado = entrada.metadata().ok().is_some_and(|metadados| {
            metadados.is_file()
                && metadados
                    .modified()
                    .ok()
                    .and_then(|modificado| modificado.elapsed().ok())
                    .is_some_and(|idade| idade > IDADE_DE_ORFAO)
        });
        if parado {
            let _ = fs::remove_file(entrada.path());
        }
    }
}

/// Compara a extensão sem diferenciar maiúsculas: `LIVRO.DOCX` vale.
fn tem_extensao(caminho: &Path, esperada: &str) -> bool {
    caminho
        .extension()
        .and_then(|extensao| extensao.to_str())
        .is_some_and(|extensao| extensao.eq_ignore_ascii_case(esperada))
}
