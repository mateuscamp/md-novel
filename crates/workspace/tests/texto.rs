//! Contagem de palavras e título do capítulo, pelos casos compartilhados com
//! a interface.

use workspace::{contar_palavras, titulo_do_texto};

/// As linhas `esperado|texto` de um arquivo de casos, com `\n` desfeito.
fn casos(arquivo: &str) -> Vec<(String, String)> {
    let casos: Vec<(String, String)> = arquivo
        .lines()
        // Comentário começa com "# "; um caso começa pelo valor esperado.
        .filter(|linha| !linha.starts_with("# "))
        .filter_map(|linha| linha.split_once('|'))
        .map(|(esperado, texto)| (esperado.to_owned(), texto.replace("\\n", "\n")))
        .collect();
    assert!(
        casos.len() >= 8,
        "esperava os casos do arquivo, li {}",
        casos.len()
    );
    casos
}

#[test]
fn conta_as_palavras_como_os_casos_compartilhados() {
    for (esperado, texto) in casos(include_str!("fixtures/contagem.txt")) {
        let esperado: usize = esperado.parse().expect("número de palavras");
        assert_eq!(contar_palavras(&texto), esperado, "texto: {texto:?}");
    }
}

#[test]
fn acha_o_titulo_como_os_casos_compartilhados() {
    for (esperado, texto) in casos(include_str!("fixtures/titulos.txt")) {
        let esperado = (!esperado.is_empty()).then_some(esperado);
        assert_eq!(titulo_do_texto(&texto), esperado, "texto: {texto:?}");
    }
}
