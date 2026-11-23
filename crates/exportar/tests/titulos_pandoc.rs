//! Os casos de título que o `workspace` e a interface compartilham
//! (`crates/workspace/tests/fixtures/titulos.txt`) conferidos com o Pandoc de
//! verdade: o título que a lateral do app mostra tem de ser o que o Pandoc
//! vai pôr no .docx. Sem este teste o arquivo de casos podia afirmar uma
//! regra que o Pandoc não segue (em 2026-11-23 ele afirmava duas).
//!
//! Precisa do `pandoc` no PATH. Sem ele o teste falha; não é pulado.

use std::fs;
use std::process::Command;

const CASOS: &str = include_str!("../../workspace/tests/fixtures/titulos.txt");

/// O texto do primeiro título não vazio, como o Pandoc leu o Markdown.
fn titulo_no_pandoc(texto: &str) -> String {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let capitulo = pasta.path().join("capitulo.md");
    fs::write(&capitulo, texto).expect("gravar o capítulo");
    // O Pandoc devolve o Markdown normalizado: todo título vira uma linha
    // `# ...`, e um `#` que era texto volta escapado (`\#`).
    let saida = Command::new("pandoc")
        .args([
            "--from=markdown",
            "--to=markdown",
            "--markdown-headings=atx",
            "--wrap=none",
            "--",
        ])
        .arg(&capitulo)
        .output()
        .expect("rodar o pandoc");
    assert!(saida.status.success(), "o pandoc falhou em {texto:?}");
    String::from_utf8(saida.stdout)
        .expect("saída em UTF-8")
        .lines()
        // Título na saída é `#`s e um espaço. Um `#semespaco` que era texto
        // volta como estava, sem o espaço.
        .filter_map(|linha| {
            let resto = linha.trim_start_matches('#');
            let e_titulo =
                resto.len() < linha.len() && (resto.is_empty() || resto.starts_with(' '));
            // A ênfase não faz parte do nome do capítulo, como no `workspace`.
            e_titulo.then(|| resto.replace('*', "").trim().to_owned())
        })
        .find(|titulo| !titulo.is_empty())
        .unwrap_or_default()
}

#[test]
fn os_casos_de_titulo_compartilhados_sao_o_que_o_pandoc_le() {
    let casos: Vec<(&str, String)> = CASOS
        .lines()
        .filter(|linha| !linha.starts_with("# "))
        .filter_map(|linha| linha.split_once('|'))
        .map(|(esperado, texto)| (esperado, texto.replace("\\n", "\n")))
        .collect();
    assert!(
        casos.len() >= 8,
        "esperava os casos do arquivo, li {}",
        casos.len()
    );
    for (esperado, texto) in casos {
        assert_eq!(titulo_no_pandoc(&texto), esperado, "texto: {texto:?}");
    }
}
