//! Exportar o livro aberto para .docx. Sem tauri aqui: `main.rs` escolhe o
//! destino (seletor do sistema) e repassa. A conversão é do crate `exportar`
//! e o estilo, do crate `estilo`; este módulo só junta os dois.

use std::path::{Path, PathBuf};

use estilo::{Estilo, gerar_reference_docx};
use exportar::{exportar_docx, modelo_reference_docx};
use serde::Serialize;

use crate::sessao::ErroDeComando;

/// O que exportar, tirado da sessão de uma vez: a conversão demora, e a
/// sessão não pode ficar travada enquanto o Pandoc roda.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanoDeExportacao {
    /// Nome do livro, para sugerir o nome do arquivo.
    pub nome: String,
    pub pasta: PathBuf,
    /// Os capítulos, na ordem do livro.
    pub capitulos: Vec<PathBuf>,
    /// O texto do `estilo.yaml`, se o livro tem um.
    pub estilo: Option<String>,
}

/// O resultado, como a interface mostra.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExportacaoFeita {
    pub destino: String,
    /// Avisos do Pandoc e do próprio app: o .docx saiu, mas o autor precisa ver.
    pub avisos: Vec<String>,
}

/// Converte o livro para .docx no destino escolhido pelo autor.
///
/// Sem `estilo.yaml` o .docx sai com o estilo padrão do Pandoc, com aviso.
/// Com `estilo.yaml` inválido nada é exportado: um .docx com a cara errada
/// passaria por certo.
pub fn exportar(
    plano: &PlanoDeExportacao,
    destino: &Path,
) -> Result<ExportacaoFeita, ErroDeComando> {
    if plano.capitulos.is_empty() {
        return Err(erro(
            "Este livro não tem nenhum capítulo para exportar: os capítulos são os arquivos .md da pasta.",
        ));
    }
    let destino = destino_final(destino)?;
    let modelo = modelo_reference_docx().map_err(|origem| erro(&origem.to_string()))?;
    let mut avisos = Vec::new();
    let reference = match &plano.estilo {
        Some(texto) => {
            let estilo = Estilo::de_yaml(texto).map_err(|origem| {
                erro(&format!(
                    "O estilo.yaml não pôde ser usado, então nada foi exportado. {origem}"
                ))
            })?;
            gerar_reference_docx(&modelo, &estilo).map_err(|origem| erro(&origem.to_string()))?
        }
        None => {
            avisos.push(
                "Esta pasta não tem estilo.yaml: o .docx saiu com o estilo padrão do Pandoc."
                    .to_owned(),
            );
            modelo
        }
    };
    let do_pandoc = exportar_docx(&plano.capitulos, &reference, &destino)
        .map_err(|origem| erro(&origem.to_string()))?;
    avisos.extend(do_pandoc);
    Ok(ExportacaoFeita {
        destino: destino.display().to_string(),
        avisos,
    })
}

/// O destino de verdade. Quem digita só "meu livro" no seletor quer
/// `meu livro.docx`: o nome que não termina em `.docx` ganha a extensão no
/// fim, sem trocar o que veio depois de um ponto do título.
///
/// O seletor do sistema só confirma a substituição do nome que foi digitado.
/// Se o nome foi completado aqui e o arquivo final já existe, ninguém
/// confirmou nada: recusa, para não gravar por cima do .docx que a editora
/// devolveu.
fn destino_final(destino: &Path) -> Result<PathBuf, ErroDeComando> {
    let ja_e_docx = destino
        .extension()
        .is_some_and(|extensao| extensao.eq_ignore_ascii_case("docx"));
    if ja_e_docx {
        return Ok(destino.to_owned());
    }
    let mut completado = destino.as_os_str().to_owned();
    completado.push(".docx");
    let completado = PathBuf::from(completado);
    if completado.exists() {
        return Err(erro(&format!(
            "Já existe {completado:?}, e você não confirmou a substituição. \
             Para substituir, escolha esse arquivo no seletor, com o .docx no nome."
        )));
    }
    Ok(completado)
}

fn erro(mensagem: &str) -> ErroDeComando {
    ErroDeComando {
        codigo: "erro",
        mensagem: mensagem.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Read};

    use super::*;
    use crate::sessao::Sessao;

    const ESTILO: &str = "versao: 1
corpo:
  fonte: EB Garamond
  tamanho: 12pt
  entrelinha: 1.5
  alinhamento: justificado
  recuo-primeira-linha: 1.25cm
  espaco-antes: 0pt
  espaco-depois: 0pt
estilos:
  Epígrafe:
    alinhamento: direita
    italico: true
";

    fn livro_com(arquivos: &[(&str, &str)]) -> (tempfile::TempDir, PlanoDeExportacao) {
        let pasta = tempfile::tempdir().expect("pasta temporária");
        for (nome, conteudo) in arquivos {
            fs::write(pasta.path().join(nome), conteudo).expect("criar arquivo");
        }
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        let plano = sessao.plano_de_exportacao().expect("plano");
        (pasta, plano)
    }

    fn entrada(docx: &Path, nome: &str) -> String {
        let bytes = fs::read(docx).unwrap_or_else(|erro| panic!("ler {}: {erro}", docx.display()));
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("o .docx é um zip");
        let mut texto = String::new();
        zip.by_name(nome)
            .expect("entrada do zip")
            .read_to_string(&mut texto)
            .expect("entrada UTF-8");
        texto
    }

    #[test]
    fn plano_traz_os_capitulos_na_ordem_do_livro_e_o_estilo() {
        let (pasta, plano) =
            livro_com(&[("02.md", "dois"), ("01.md", "um"), ("estilo.yaml", ESTILO)]);
        let real = pasta.path().canonicalize().expect("caminho real");
        assert_eq!(plano.capitulos, [real.join("01.md"), real.join("02.md")]);
        assert_eq!(plano.estilo.as_deref(), Some(ESTILO));
        assert_eq!(plano.pasta, real);
        assert_eq!(
            plano.nome,
            real.file_name().expect("nome").to_string_lossy()
        );
    }

    #[test]
    fn sem_livro_aberto_nao_ha_plano() {
        let erro = Sessao::default()
            .plano_de_exportacao()
            .expect_err("sem livro");
        assert!(erro.mensagem.contains("pasta"), "{erro:?}");
    }

    #[test]
    fn exporta_os_capitulos_em_ordem_com_o_estilo_do_livro() {
        let (pasta, plano) = livro_com(&[
            ("02.md", "# Dois\n\nTexto do segundo.\n"),
            (
                "01.md",
                "# Um\n\n::: {custom-style=\"Epígrafe\"}\nA epígrafe.\n:::\n\nTexto do primeiro.\n",
            ),
            ("estilo.yaml", ESTILO),
        ]);
        let destino = pasta.path().join("livro.docx");

        let feita = exportar(&plano, &destino).expect("exportar");

        assert_eq!(feita.destino, destino.display().to_string());
        assert_eq!(feita.avisos, Vec::<String>::new());
        let documento = entrada(&destino, "word/document.xml");
        let primeiro = documento.find("Texto do primeiro").expect("capítulo 1");
        let segundo = documento.find("Texto do segundo").expect("capítulo 2");
        assert!(primeiro < segundo, "na ordem do livro");
        assert!(
            documento.contains(r#"<w:pStyle w:val="Epígrafe""#),
            "{documento}"
        );
        let estilos = entrada(&destino, "word/styles.xml");
        assert!(
            estilos.contains(r#"<w:jc w:val="both""#),
            "o corpo do estilo.yaml"
        );
        // A pasta do livro ganha só o .docx pedido.
        let mut nomes: Vec<String> = fs::read_dir(pasta.path())
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
        assert_eq!(nomes, ["01.md", "02.md", "estilo.yaml", "livro.docx"]);
    }

    // Até a revisão de 2026-11-23 este teste esperava erro para `01.md`. A
    // regra mudou: nome que não termina em .docx ganha `.docx` no fim, então
    // `01.md` vira `01.md.docx` e o capítulo continua intacto. A recusa por
    // extensão errada segue valendo na linha de comando (crates/cli).
    #[test]
    fn nome_que_nao_termina_em_docx_ganha_a_extensao_no_fim() {
        let (pasta, plano) = livro_com(&[("01.md", "Texto.\n"), ("estilo.yaml", ESTILO)]);
        for (digitado, esperado) in [
            ("meu livro", "meu livro.docx"),
            ("O Dom da Pele v2.1", "O Dom da Pele v2.1.docx"),
            ("01.md", "01.md.docx"),
            ("Final.DOCX", "Final.DOCX"),
        ] {
            let feita = exportar(&plano, &pasta.path().join(digitado))
                .unwrap_or_else(|erro| panic!("{digitado}: {erro:?}"));
            assert_eq!(
                feita.destino,
                pasta.path().join(esperado).display().to_string()
            );
            assert!(pasta.path().join(esperado).exists(), "{esperado}");
        }
        assert_eq!(
            fs::read_to_string(pasta.path().join("01.md")).expect("reler"),
            "Texto.\n"
        );
    }

    #[test]
    fn nome_completado_nao_substitui_um_docx_que_o_seletor_nao_confirmou() {
        let (pasta, plano) = livro_com(&[("01.md", "Texto.\n"), ("estilo.yaml", ESTILO)]);
        let da_editora = pasta.path().join("livro.docx");
        fs::write(&da_editora, "revisão da editora").expect("docx existente");

        let erro = exportar(&plano, &pasta.path().join("livro")).expect_err("já existe livro.docx");

        assert!(erro.mensagem.contains("livro.docx"), "{erro:?}");
        assert!(erro.mensagem.contains("seletor"), "{erro:?}");
        assert_eq!(
            fs::read_to_string(&da_editora).expect("reler"),
            "revisão da editora"
        );

        // Escolhido com o nome completo, o seletor já perguntou: substitui.
        exportar(&plano, &da_editora).expect("substituir com confirmação");
        assert!(entrada(&da_editora, "word/document.xml").contains("Texto."));
    }

    #[test]
    fn livro_sem_estilo_exporta_com_o_padrao_do_pandoc_e_avisa() {
        let (pasta, plano) = livro_com(&[("01.md", "# Um\n\nTexto.\n")]);
        let destino = pasta.path().join("livro.docx");

        let feita = exportar(&plano, &destino).expect("exportar");

        assert!(entrada(&destino, "word/document.xml").contains("Texto."));
        assert_eq!(feita.avisos.len(), 1, "{:?}", feita.avisos);
        assert!(
            feita.avisos[0].contains("estilo.yaml"),
            "{:?}",
            feita.avisos
        );
    }

    #[test]
    fn estilo_invalido_nao_exporta_um_docx_com_a_cara_errada() {
        let ruim = ESTILO.replace("1.25cm", "1,25cm");
        let (pasta, plano) = livro_com(&[("01.md", "Texto.\n"), ("estilo.yaml", &ruim)]);
        let destino = pasta.path().join("livro.docx");

        let erro = exportar(&plano, &destino).expect_err("estilo inválido");

        assert!(erro.mensagem.contains("1,25cm"), "{erro:?}");
        assert!(!destino.exists());
    }

    #[test]
    fn livro_sem_capitulo_nao_exporta() {
        let (pasta, plano) = livro_com(&[("estilo.yaml", ESTILO)]);
        let erro = exportar(&plano, &pasta.path().join("livro.docx")).expect_err("sem capítulos");
        // A mensagem é a do app, que explica o que é capítulo, e não a do
        // crate `exportar`, que só chegaria depois de chamar o Pandoc.
        assert!(erro.mensagem.contains("arquivos .md da pasta"), "{erro:?}");
        assert!(!pasta.path().join("livro.docx").exists());
    }

    #[test]
    fn aviso_do_pandoc_chega_ao_resultado() {
        let (pasta, plano) = livro_com(&[
            ("01.md", "![Sumiu](nao-existe.png)\n"),
            ("estilo.yaml", ESTILO),
        ]);
        let feita = exportar(&plano, &pasta.path().join("livro.docx")).expect("exportar");
        assert!(
            feita
                .avisos
                .iter()
                .any(|aviso| aviso.contains("nao-existe.png")),
            "{:?}",
            feita.avisos
        );
    }
}
