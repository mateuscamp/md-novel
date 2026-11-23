//! Único lugar do projeto que chama o Pandoc.
//!
//! A conversão é só de ida, de .md para .docx (ADR 0001): aqui não entra
//! leitura de .docx.

use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use thiserror::Error;

use workspace::{ErroWorkspace, SaidaPendente};

#[derive(Debug, Error)]
pub enum ErroExportar {
    #[error("não achei o pandoc no PATH; instale o Pandoc 3 ou mais novo")]
    PandocAusente,

    #[error("o pandoc falhou (código {codigo}): {stderr}")]
    PandocFalhou { codigo: String, stderr: String },

    #[error("o pandoc terminou sem erro mas não gravou o .docx; nada foi alterado no destino")]
    PandocNaoGravou,

    #[error("nenhum capítulo para exportar; esperado ao menos um arquivo .md")]
    SemCapitulos,

    #[error("não achei o capítulo {recebido:?}; confira o nome e a pasta")]
    CapituloInexistente { recebido: PathBuf },

    #[error(
        "o capítulo {recebido:?} não é um arquivo; se for uma pasta, passe os .md de dentro dela"
    )]
    CapituloNaoEArquivo { recebido: PathBuf },

    /// Problema com o destino do .docx (extensão, pasta, link): a mensagem
    /// é a do crate `workspace`, que é quem grava.
    #[error(transparent)]
    Saida(#[from] ErroWorkspace),

    #[error("erro de leitura ou escrita: {0}")]
    Io(#[from] std::io::Error),
}

/// O reference.docx padrão do Pandoc instalado, que `estilo::gerar_reference_docx`
/// usa como modelo.
pub fn modelo_reference_docx() -> Result<Vec<u8>, ErroExportar> {
    let argumentos = ["--print-default-data-file", "reference.docx"].map(OsString::from);
    Ok(rodar_pandoc(&argumentos)?.saida_padrao)
}

/// Converte os capítulos, na ordem dada, num único .docx com os estilos do
/// `reference_docx`. A saída só aparece no destino quando está completa.
///
/// Devolve os avisos do Pandoc (imagem não encontrada, por exemplo): a
/// exportação deu certo, mas o autor precisa vê-los.
pub fn exportar_docx(
    capitulos: &[PathBuf],
    reference_docx: &[u8],
    saida: &Path,
) -> Result<Vec<String>, ErroExportar> {
    if capitulos.is_empty() {
        // Sem arquivo de entrada o Pandoc ficaria lendo a entrada padrão.
        return Err(ErroExportar::SemCapitulos);
    }
    // Conferido aqui para o erro ser o do projeto, com o caminho, e não a
    // mensagem crua do Pandoc.
    if let Some(falta) = capitulos.iter().find(|capitulo| !capitulo.exists()) {
        return Err(ErroExportar::CapituloInexistente {
            recebido: falta.clone(),
        });
    }
    if let Some(pasta) = capitulos.iter().find(|capitulo| !capitulo.is_file()) {
        return Err(ErroExportar::CapituloNaoEArquivo {
            recebido: pasta.clone(),
        });
    }
    let pendente = SaidaPendente::preparar(saida, "docx")?;

    // `tempfile` cria com nome imprevisível e permissão 0600.
    let mut reference = tempfile::Builder::new()
        .prefix("md-novel-reference-")
        .suffix(".docx")
        .tempfile()?;
    reference.write_all(reference_docx)?;
    reference.flush()?;

    let argumentos =
        argumentos_de_exportacao(reference.path(), pendente.caminho_temporario(), capitulos);
    let SaidaDoPandoc { avisos, .. } = rodar_pandoc(&argumentos)?;
    // O código de saída não basta: um Pandoc que termina bem sem gravar
    // trocaria o .docx anterior por um arquivo vazio.
    if fs::metadata(pendente.caminho_temporario())?.len() == 0 {
        return Err(ErroExportar::PandocNaoGravou);
    }
    pendente.publicar()?;
    Ok(avisos)
}

/// Argumentos da conversão. Fica separado para o teste do ADR 0001 conferir
/// que o Pandoc só lê Markdown.
fn argumentos_de_exportacao(
    reference: &Path,
    saida: &Path,
    capitulos: &[PathBuf],
) -> Vec<OsString> {
    let com_valor = |opcao: &str, valor: &Path| {
        let mut argumento = OsString::from(opcao);
        argumento.push(valor);
        argumento
    };
    let mut argumentos = vec![
        // `rebase_relative_paths`: o caminho de cada imagem vale a partir da
        // pasta do capítulo que a cita, como no Zettlr. Uma lista única de
        // pastas (`--resource-path`) trocava imagens de mesmo nome entre
        // capítulos de pastas diferentes.
        OsString::from("--from=markdown+rebase_relative_paths"),
        OsString::from("--to=docx"),
        // Cada capítulo é interpretado sozinho: sem isto, a nota `[^1]` de um
        // capítulo sobrescreve a `[^1]` de outro.
        OsString::from("--file-scope"),
        com_valor("--reference-doc=", reference),
        com_valor("--output=", saida),
        // Depois de `--` nada é opção: um capítulo chamado `-x.md` não vira flag.
        OsString::from("--"),
    ];
    argumentos.extend(
        capitulos
            .iter()
            .map(|capitulo| capitulo.clone().into_os_string()),
    );
    argumentos
}

struct SaidaDoPandoc {
    saida_padrao: Vec<u8>,
    /// Linhas que o Pandoc escreveu no erro padrão tendo terminado com sucesso.
    avisos: Vec<String>,
}

/// Roda o Pandoc sem shell e sem entrada padrão.
fn rodar_pandoc(argumentos: &[OsString]) -> Result<SaidaDoPandoc, ErroExportar> {
    let resultado = Command::new("pandoc")
        .args(argumentos)
        .stdin(Stdio::null())
        .output()
        .map_err(|erro| {
            if erro.kind() == ErrorKind::NotFound {
                ErroExportar::PandocAusente
            } else {
                ErroExportar::Io(erro)
            }
        })?;
    let erro_padrao = String::from_utf8_lossy(&resultado.stderr);
    if !resultado.status.success() {
        return Err(ErroExportar::PandocFalhou {
            codigo: resultado.status.code().map_or_else(
                || "encerrado por sinal".to_owned(),
                |codigo| codigo.to_string(),
            ),
            stderr: erro_padrao.trim().to_owned(),
        });
    }
    let avisos = erro_padrao
        .lines()
        .map(str::trim_end)
        .filter(|linha| !linha.trim().is_empty())
        .map(str::to_owned)
        .collect();
    Ok(SaidaDoPandoc {
        saida_padrao: resultado.stdout,
        avisos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn como_texto(argumentos: &[OsString]) -> Vec<String> {
        argumentos
            .iter()
            .map(|argumento| argumento.to_string_lossy().into_owned())
            .collect()
    }

    // Expectativa trocada duas vezes em 2026-11-23. Primeira auditoria: entrou
    // `--file-scope` (notas) e `--resource-path` (imagens). Segunda: o
    // `--resource-path` saiu e entrou `+rebase_relative_paths`, porque uma
    // lista única de pastas trocava imagens de mesmo nome entre capítulos.
    // Nenhuma das listas antigas vale mais em modo nenhum.
    #[test]
    fn pandoc_le_markdown_e_escreve_docx_com_os_capitulos_depois_do_separador() {
        let argumentos = argumentos_de_exportacao(
            Path::new("/tmp/ref.docx"),
            Path::new("/tmp/livro.docx"),
            &[
                PathBuf::from("-cap1.md"),
                PathBuf::from("livro/cap2.md"),
                PathBuf::from("livro/cap3.md"),
            ],
        );
        assert_eq!(
            como_texto(&argumentos),
            [
                "--from=markdown+rebase_relative_paths",
                "--to=docx",
                "--file-scope",
                "--reference-doc=/tmp/ref.docx",
                "--output=/tmp/livro.docx",
                "--",
                "-cap1.md",
                "livro/cap2.md",
                "livro/cap3.md"
            ]
        );
    }

    // O teste `pasta_de_capitulo_com_dois_pontos_e_recusada...` saiu em
    // 2026-11-23 junto com a regra: sem `--resource-path` não há lista de
    // pastas, e pasta com dois-pontos passou a funcionar (coberto em
    // tests/exportacao.rs, com a pasta "parte 2: fim").

    // Variante renomeada em 2026-11-23: a guarda de extensão passou a valer
    // também para o CSS, então `SaidaNaoEDocx` virou `ExtensaoErrada`.
    #[test]
    fn saida_que_nao_e_docx_e_recusada_para_nunca_sobrescrever_um_capitulo() {
        let pasta = tempfile::tempdir().expect("pasta temporária");
        let capitulo = pasta.path().join("cap1.md");
        std::fs::write(&capitulo, "texto do autor").expect("criar capítulo");
        for saida in [
            capitulo.clone(),
            pasta.path().join("livro"),
            pasta.path().join("livro.md"),
        ] {
            let erro = exportar_docx(std::slice::from_ref(&capitulo), b"", &saida)
                .expect_err("saída recusada");
            assert!(
                matches!(
                    erro,
                    ErroExportar::Saida(ErroWorkspace::ExtensaoErrada { .. })
                ),
                "{}: veio {erro}",
                saida.display()
            );
        }
        assert_eq!(
            std::fs::read_to_string(&capitulo).expect("reler capítulo"),
            "texto do autor"
        );
    }

    #[test]
    fn exportar_sem_capitulo_e_erro_em_vez_de_esperar_a_entrada_padrao() {
        let erro =
            exportar_docx(&[], b"", Path::new("/tmp/nao-usado.docx")).expect_err("sem capítulos");
        assert!(matches!(erro, ErroExportar::SemCapitulos), "veio {erro}");
    }
}
