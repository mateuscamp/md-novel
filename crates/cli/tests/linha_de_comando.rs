//! Testes funcionais da linha de comando: rodam o binário de verdade numa
//! pasta temporária, como o autor faria.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ROMANCE: &str = include_str!("../../estilo/tests/fixtures/romance.yaml");
const CSS_ESPERADO: &str = include_str!("../../estilo/tests/golden/romance.css");

fn md_novel(pasta: &Path, argumentos: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_md-novel"))
        .args(argumentos)
        .current_dir(pasta)
        .output()
        .expect("rodar o binário md-novel")
}

fn pasta_com_romance() -> (tempfile::TempDir, PathBuf) {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let yaml = pasta.path().join("estilo.yaml");
    fs::write(&yaml, ROMANCE).expect("gravar estilo.yaml");
    (pasta, yaml)
}

fn entrada_do_zip(caminho: &Path, nome: &str) -> String {
    let bytes =
        fs::read(caminho).unwrap_or_else(|erro| panic!("ler {}: {erro}", caminho.display()));
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("a saída é um zip");
    let mut texto = String::new();
    zip.by_name(nome)
        .expect("entrada do zip")
        .read_to_string(&mut texto)
        .expect("entrada UTF-8");
    texto
}

fn nomes_na_pasta(pasta: &Path) -> Vec<String> {
    let mut nomes: Vec<String> = fs::read_dir(pasta)
        .expect("listar pasta")
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

#[test]
fn estilo_gera_o_css_e_o_reference_docx() {
    let (pasta, _) = pasta_com_romance();
    let saida = md_novel(
        pasta.path(),
        &[
            "estilo",
            "estilo.yaml",
            "--css",
            "editor.css",
            "--docx",
            "reference.docx",
        ],
    );
    assert!(
        saida.status.success(),
        "{}",
        String::from_utf8_lossy(&saida.stderr)
    );
    assert_eq!(
        fs::read_to_string(pasta.path().join("editor.css")).expect("ler o css"),
        CSS_ESPERADO
    );
    let styles = entrada_do_zip(&pasta.path().join("reference.docx"), "word/styles.xml");
    assert!(styles.contains(r#"<w:name w:val="Epígrafe"/>"#), "{styles}");
    assert_eq!(
        nomes_na_pasta(pasta.path()),
        ["editor.css", "estilo.yaml", "reference.docx"],
        "só as saídas pedidas, sem temporário"
    );
    assert_eq!(
        fs::read_to_string(pasta.path().join("estilo.yaml")).expect("reler"),
        ROMANCE
    );
}

#[test]
fn estilo_aceita_o_seletor_de_corpo_de_outro_editor() {
    let (pasta, _) = pasta_com_romance();
    let saida = md_novel(
        pasta.path(),
        &[
            "estilo",
            "estilo.yaml",
            "--css",
            "zettlr.css",
            "--seletor-corpo",
            ".cm-line",
        ],
    );
    assert!(
        saida.status.success(),
        "{}",
        String::from_utf8_lossy(&saida.stderr)
    );
    let css = fs::read_to_string(pasta.path().join("zettlr.css")).expect("ler o css");
    assert!(css.contains("\n.cm-line {\n"), "{css}");
}

#[test]
fn estilo_invalido_falha_com_a_causa_e_nao_cria_arquivo() {
    let (pasta, yaml) = pasta_com_romance();
    fs::write(&yaml, ROMANCE.replace("1.25cm", "1,25cm")).expect("gravar yaml inválido");
    let saida = md_novel(
        pasta.path(),
        &["estilo", "estilo.yaml", "--css", "editor.css"],
    );
    assert_eq!(saida.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&saida.stderr);
    assert!(stderr.contains("1,25cm"), "{stderr}");
    assert_eq!(nomes_na_pasta(pasta.path()), ["estilo.yaml"]);
}

#[test]
fn estilo_sem_nenhuma_saida_pedida_e_erro() {
    let (pasta, _) = pasta_com_romance();
    let saida = md_novel(pasta.path(), &["estilo", "estilo.yaml"]);
    assert!(!saida.status.success());
    assert_eq!(nomes_na_pasta(pasta.path()), ["estilo.yaml"]);
}

#[test]
fn saida_com_extensao_errada_e_recusada_e_nada_e_sobrescrito() {
    let (pasta, _) = pasta_com_romance();
    fs::write(pasta.path().join("cap1.md"), "texto do autor").expect("criar capítulo");
    for argumentos in [
        ["estilo", "estilo.yaml", "--css", "estilo.yaml"],
        ["estilo", "estilo.yaml", "--css", "cap1.md"],
        ["estilo", "estilo.yaml", "--docx", "cap1.md"],
    ] {
        let saida = md_novel(pasta.path(), &argumentos);
        assert_eq!(saida.status.code(), Some(1), "{argumentos:?}");
    }
    let saida = md_novel(
        pasta.path(),
        &[
            "exportar",
            "--estilo",
            "estilo.yaml",
            "--saida",
            "cap1.md",
            "cap1.md",
        ],
    );
    assert_eq!(saida.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(pasta.path().join("estilo.yaml")).expect("reler"),
        ROMANCE
    );
    assert_eq!(
        fs::read_to_string(pasta.path().join("cap1.md")).expect("reler"),
        "texto do autor"
    );
    assert_eq!(nomes_na_pasta(pasta.path()), ["cap1.md", "estilo.yaml"]);
}

#[test]
fn exportar_junta_os_capitulos_na_ordem_com_o_estilo_do_livro() {
    let (pasta, _) = pasta_com_romance();
    fs::write(pasta.path().join("cap1.md"), "# Um\n\nTexto do primeiro.\n").expect("cap1");
    fs::write(
        pasta.path().join("cap2.md"),
        "# Dois\n\n::: {custom-style=\"Epígrafe\"}\nUma epígrafe.\n:::\n",
    )
    .expect("cap2");
    let saida = md_novel(
        pasta.path(),
        &[
            "exportar",
            "--estilo",
            "estilo.yaml",
            "--saida",
            "livro.docx",
            "cap1.md",
            "cap2.md",
        ],
    );
    assert!(
        saida.status.success(),
        "{}",
        String::from_utf8_lossy(&saida.stderr)
    );
    let livro = pasta.path().join("livro.docx");
    let document = entrada_do_zip(&livro, "word/document.xml");
    let primeiro = document.find("Texto do primeiro").expect("texto do cap1");
    let segundo = document.find("Uma epígrafe").expect("texto do cap2");
    assert!(primeiro < segundo, "os capítulos saem na ordem dada");
    assert!(
        document.contains(r#"<w:pStyle w:val="Epígrafe""#),
        "{document}"
    );
    let styles = entrada_do_zip(&livro, "word/styles.xml");
    assert!(styles.contains(r#"<w:jc w:val="both""#), "{styles}");
    assert_eq!(
        nomes_na_pasta(pasta.path()),
        ["cap1.md", "cap2.md", "estilo.yaml", "livro.docx"]
    );
}

// Regressões dos achados da auditoria de 2026-11-23.

#[test]
fn se_uma_saida_nao_pode_ser_gravada_nenhuma_e_trocada() {
    let (pasta, _) = pasta_com_romance();
    fs::write(pasta.path().join("editor.css"), "ANTIGO").expect("css antigo");
    let saida = md_novel(
        pasta.path(),
        &[
            "estilo",
            "estilo.yaml",
            "--css",
            "editor.css",
            "--docx",
            "nao-existe/reference.docx",
        ],
    );
    assert_eq!(saida.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&saida.stderr);
    assert!(stderr.contains("nao-existe"), "{stderr}");
    assert_eq!(
        fs::read_to_string(pasta.path().join("editor.css")).expect("reler o css"),
        "ANTIGO",
        "o CSS novo não pode ficar ao lado do reference.docx antigo"
    );
    assert_eq!(nomes_na_pasta(pasta.path()), ["editor.css", "estilo.yaml"]);
}

#[test]
fn exportar_mostra_os_avisos_do_pandoc() {
    let (pasta, _) = pasta_com_romance();
    fs::write(
        pasta.path().join("cap1.md"),
        "# Um\n\n![Sumiu](nao-existe.png)\n",
    )
    .expect("cap1");
    let saida = md_novel(
        pasta.path(),
        &[
            "exportar",
            "--estilo",
            "estilo.yaml",
            "--saida",
            "livro.docx",
            "cap1.md",
        ],
    );
    assert!(saida.status.success(), "aviso não é erro");
    let stderr = String::from_utf8_lossy(&saida.stderr);
    assert!(stderr.contains("aviso do Pandoc:"), "{stderr}");
    assert!(stderr.contains("nao-existe.png"), "{stderr}");
    assert!(pasta.path().join("livro.docx").exists());
}

#[test]
fn estilo_yaml_acima_de_1_mib_e_recusado() {
    let (pasta, yaml) = pasta_com_romance();
    let enchimento = format!("{ROMANCE}# {}\n", "x".repeat(1024 * 1024));
    fs::write(&yaml, enchimento).expect("yaml enorme");
    let saida = md_novel(
        pasta.path(),
        &["estilo", "estilo.yaml", "--css", "editor.css"],
    );
    assert_eq!(saida.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&saida.stderr);
    assert!(stderr.contains("1 MiB"), "{stderr}");
    assert_eq!(nomes_na_pasta(pasta.path()), ["estilo.yaml"]);
}

#[cfg(unix)]
#[test]
fn css_existente_mantem_a_permissao_ao_ser_substituido() {
    use std::os::unix::fs::PermissionsExt;

    let (pasta, _) = pasta_com_romance();
    let css = pasta.path().join("editor.css");
    fs::write(&css, "ANTIGO").expect("css antigo");
    fs::set_permissions(&css, fs::Permissions::from_mode(0o644)).expect("permissão");
    let saida = md_novel(
        pasta.path(),
        &["estilo", "estilo.yaml", "--css", "editor.css"],
    );
    assert!(
        saida.status.success(),
        "{}",
        String::from_utf8_lossy(&saida.stderr)
    );
    let modo = fs::metadata(&css).expect("metadados").permissions().mode() & 0o777;
    assert_eq!(modo, 0o644, "veio {modo:o}");
    assert_eq!(fs::read_to_string(&css).expect("ler o css"), CSS_ESPERADO);
}

// Regressão da terceira auditoria de 2026-11-23.

/// Caminho do Pandoc de verdade, para o falso poder entregar o modelo.
#[cfg(unix)]
fn pandoc_de_verdade() -> PathBuf {
    let caminhos = std::env::var_os("PATH").expect("PATH definido");
    std::env::split_paths(&caminhos)
        .map(|pasta| pasta.join("pandoc"))
        .find(|candidato| candidato.is_file())
        .expect("pandoc no PATH")
}

/// Roda `exportar` com um pandoc falso no PATH: ele entrega o modelo de
/// verdade e, na conversão, sai com 0 sem escrever nada. O falso fica fora da
/// pasta do livro para não entrar na listagem dela.
#[cfg(unix)]
fn exportar_com_pandoc_que_nao_grava(pasta: &Path) -> Output {
    use std::os::unix::fs::PermissionsExt;

    let falsos = tempfile::tempdir().expect("pasta do pandoc falso");
    let falso = falsos.path().join("pandoc");
    let roteiro = format!(
        "#!/bin/sh\ncase \"$1\" in --print-default-data-file) exec '{}' \"$@\";; esac\nexit 0\n",
        pandoc_de_verdade().display()
    );
    fs::write(&falso, roteiro).expect("gravar o pandoc falso");
    fs::set_permissions(&falso, fs::Permissions::from_mode(0o755)).expect("tornar executável");
    Command::new(env!("CARGO_BIN_EXE_md-novel"))
        .args([
            "exportar",
            "--estilo",
            "estilo.yaml",
            "--saida",
            "livro.docx",
            "cap1.md",
        ])
        .current_dir(pasta)
        .env("PATH", falsos.path())
        .output()
        .expect("rodar o binário md-novel")
}

#[cfg(unix)]
#[test]
fn pandoc_que_termina_bem_sem_gravar_nao_apaga_o_docx_anterior() {
    let (pasta, _) = pasta_com_romance();
    fs::write(pasta.path().join("cap1.md"), "# Um\n\nTexto.\n").expect("cap1");
    fs::write(pasta.path().join("livro.docx"), "VERSAO ANTERIOR").expect("docx anterior");

    let saida = exportar_com_pandoc_que_nao_grava(pasta.path());

    assert_eq!(saida.status.code(), Some(1), "saída vazia é erro");
    let stderr = String::from_utf8_lossy(&saida.stderr);
    assert!(stderr.contains("não gravou"), "{stderr}");
    assert_eq!(
        fs::read_to_string(pasta.path().join("livro.docx")).expect("reler o docx"),
        "VERSAO ANTERIOR"
    );
    assert_eq!(
        nomes_na_pasta(pasta.path()),
        ["cap1.md", "estilo.yaml", "livro.docx"]
    );
}

// Regressão da quarta revisão de 2026-11-23: a mensagem falava em "arquivo
// anterior" mesmo quando não havia nenhum.
#[cfg(unix)]
#[test]
fn na_primeira_exportacao_o_erro_nao_cita_arquivo_anterior() {
    let (pasta, _) = pasta_com_romance();
    fs::write(pasta.path().join("cap1.md"), "# Um\n\nTexto.\n").expect("cap1");

    let saida = exportar_com_pandoc_que_nao_grava(pasta.path());

    assert_eq!(saida.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&saida.stderr);
    assert!(stderr.contains("não gravou"), "{stderr}");
    assert!(
        !stderr.contains("anterior"),
        "não havia arquivo anterior: {stderr}"
    );
    assert!(stderr.contains("nada foi alterado no destino"), "{stderr}");
    assert_eq!(nomes_na_pasta(pasta.path()), ["cap1.md", "estilo.yaml"]);
}
