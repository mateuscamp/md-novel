//! Governança do ADR 0001: o app só converte de .md para .docx. Este teste
//! reprova qualquer fonte do projeto que peça ao Pandoc para LER .docx.

use std::fs;
use std::path::{Path, PathBuf};

fn fontes_rust(pasta: &Path, achados: &mut Vec<PathBuf>) {
    for entrada in fs::read_dir(pasta).expect("listar pasta de fontes") {
        let caminho = entrada.expect("entrada da pasta").path();
        if caminho.is_dir() {
            fontes_rust(&caminho, achados);
        } else if caminho.extension().is_some_and(|extensao| extensao == "rs") {
            achados.push(caminho);
        }
    }
}

#[test]
fn nenhum_fonte_pede_ao_pandoc_para_ler_docx() {
    // Montadas em pedaços para este arquivo não reprovar a si mesmo.
    let proibidos = [
        ["--from", "=docx"].concat(),
        ["-f", " docx"].concat(),
        ["\"-f\", ", "\"docx"].concat(),
        ["--track", "-changes"].concat(),
        ["docx", "+styles"].concat(),
    ];
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut fontes = Vec::new();
    for nome in ["estilo", "exportar", "cli"] {
        fontes_rust(&crates.join(nome).join("src"), &mut fontes);
    }
    assert!(
        fontes.len() >= 7,
        "esperava os fontes dos três crates, achei {}",
        fontes.len()
    );
    for fonte in fontes {
        let codigo = fs::read_to_string(&fonte).expect("ler fonte");
        for proibido in &proibidos {
            assert!(
                !codigo.contains(proibido.as_str()),
                "{} contém {proibido:?}: importar .docx exige um ADR que substitua o 0001",
                fonte.display()
            );
        }
    }
}
