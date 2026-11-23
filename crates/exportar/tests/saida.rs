//! Publicação de arquivo gerado (`SaidaPendente`): regressões da segunda
//! auditoria de 2026-11-23.

use std::fs::{self, File};
use std::path::Path;
use std::time::{Duration, SystemTime};

use exportar::{ErroExportar, SaidaPendente};

fn publicar(destino: &Path, extensao: &str, conteudo: &[u8]) -> Result<(), ErroExportar> {
    let mut saida = SaidaPendente::preparar(destino, extensao)?;
    saida.escrever(conteudo)?;
    saida.publicar()
}

fn nomes(pasta: &Path) -> Vec<String> {
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

fn envelhecer(caminho: &Path, idade: Duration) {
    // Só leitura: mudar a data não pede escrita, e pasta não abre para escrita.
    File::open(caminho)
        .expect("abrir para mudar a data")
        .set_modified(SystemTime::now() - idade)
        .expect("mudar a data");
}

#[cfg(unix)]
#[test]
fn destino_que_e_link_atualiza_o_arquivo_real_e_o_link_continua_link() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let real = pasta.path().join("real");
    fs::create_dir(&real).expect("pasta real");
    fs::write(real.join("editor.css"), "ANTIGO").expect("css real");
    let link = pasta.path().join("editor.css");
    std::os::unix::fs::symlink("real/editor.css", &link).expect("criar link");

    publicar(&link, "css", b"NOVO").expect("publicar pelo link");

    assert_eq!(
        fs::read_to_string(real.join("editor.css")).expect("ler o real"),
        "NOVO"
    );
    assert!(
        fs::symlink_metadata(&link)
            .expect("metadados do link")
            .file_type()
            .is_symlink(),
        "o link não pode virar arquivo comum"
    );
    assert_eq!(nomes(&real), ["editor.css"], "sem temporário na pasta real");
    assert_eq!(nomes(pasta.path()), ["editor.css", "real"]);
}

#[cfg(unix)]
#[test]
fn link_com_nome_css_que_aponta_para_um_capitulo_e_recusado() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let capitulo = pasta.path().join("cap1.md");
    fs::write(&capitulo, "texto do autor").expect("capítulo");
    let link = pasta.path().join("editor.css");
    std::os::unix::fs::symlink("cap1.md", &link).expect("criar link");

    let erro = publicar(&link, "css", b"NOVO").expect_err("alvo não é .css");

    assert!(
        matches!(erro, ErroExportar::ExtensaoErrada { .. }),
        "veio {erro}"
    );
    assert_eq!(
        fs::read_to_string(&capitulo).expect("reler"),
        "texto do autor"
    );
}

#[cfg(unix)]
#[test]
fn link_quebrado_e_recusado_com_o_caminho_na_mensagem() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let link = pasta.path().join("editor.css");
    std::os::unix::fs::symlink("sumiu/editor.css", &link).expect("criar link");

    let erro = publicar(&link, "css", b"NOVO").expect_err("link quebrado");

    assert!(
        matches!(erro, ErroExportar::SaidaInvalida { .. }),
        "veio {erro}"
    );
    assert!(erro.to_string().contains("editor.css"), "{erro}");
}

#[test]
fn temporario_orfao_de_uma_execucao_interrompida_e_recolhido() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let orfao = pasta.path().join(".md-novel-gravando-ab12CD.docx");
    let recente = pasta.path().join(".md-novel-gravando-zz99ZZ.docx");
    let do_autor = pasta.path().join("capitulo antigo.md");
    let parecido = pasta.path().join(".md-novel-gravando");
    // Cada um destes prende uma trava da faxina (terceira auditoria de
    // 2026-11-23): prefixo sem o hífen final, prefixo certo com outra
    // extensão, e pasta com o nome inteiro de um temporário.
    let prefixo_quase = pasta.path().join(".md-novel-gravandoX.docx");
    let outra_extensao = pasta.path().join(".md-novel-gravando-notas.md");
    let pasta_parecida = pasta.path().join(".md-novel-gravando-pasta.docx");
    for arquivo in [
        &orfao,
        &recente,
        &do_autor,
        &parecido,
        &prefixo_quase,
        &outra_extensao,
    ] {
        fs::write(arquivo, "x").expect("criar arquivo");
    }
    fs::create_dir(&pasta_parecida).expect("criar pasta");
    for antigo in [
        &orfao,
        &do_autor,
        &parecido,
        &prefixo_quase,
        &outra_extensao,
        &pasta_parecida,
    ] {
        envelhecer(antigo, Duration::from_secs(2 * 60 * 60));
    }

    publicar(&pasta.path().join("livro.docx"), "docx", b"NOVO").expect("publicar");

    assert_eq!(
        nomes(pasta.path()),
        [
            ".md-novel-gravando",
            ".md-novel-gravando-notas.md",
            ".md-novel-gravando-pasta.docx",
            ".md-novel-gravando-zz99ZZ.docx",
            ".md-novel-gravandoX.docx",
            "capitulo antigo.md",
            "livro.docx"
        ],
        "só o órfão antigo, com o prefixo exato do md-novel, pode sumir"
    );
}
