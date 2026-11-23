//! Fitness function de fronteira: `estilo` é Rust puro. Não depende de
//! tauri nem dos outros crates do projeto, e não abre processo nem rede.

use std::fs;
use std::path::Path;

const DEPENDENCIAS_PERMITIDAS: [&str; 5] =
    ["serde", "serde_yaml_ng", "quick-xml", "zip", "thiserror"];
const PROIBIDO_NO_CODIGO: [&str; 3] = ["std::process", "std::net", "Command::new"];

#[test]
fn cargo_toml_so_tem_dependencias_permitidas() {
    let manifesto = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("ler o Cargo.toml do crate");
    let secao = manifesto
        .split("[dependencies]")
        .nth(1)
        .expect("seção [dependencies]")
        .split("\n[")
        .next()
        .expect("fim da seção");
    let dependencias: Vec<&str> = secao
        .lines()
        .filter_map(|linha| linha.split(['.', '=', ' ']).next())
        .filter(|nome| !nome.is_empty() && !nome.starts_with('#'))
        .collect();
    assert!(
        !dependencias.is_empty(),
        "não achei dependências em:\n{secao}"
    );
    for nome in dependencias {
        assert!(
            DEPENDENCIAS_PERMITIDAS.contains(&nome),
            "dependência {nome:?} não é permitida em crates/estilo; mude o ADR 0002 antes"
        );
    }
}

#[test]
fn codigo_nao_abre_processo_nem_rede() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut lidos = 0;
    for arquivo in fs::read_dir(&src).expect("listar src") {
        let caminho = arquivo.expect("entrada de src").path();
        let codigo = fs::read_to_string(&caminho).expect("ler fonte");
        lidos += 1;
        for proibido in PROIBIDO_NO_CODIGO {
            assert!(
                !codigo.contains(proibido),
                "{} usa {proibido}; processo externo só em crates/exportar",
                caminho.display()
            );
        }
    }
    assert!(lidos >= 5, "esperava ler os módulos de src, li {lidos}");
}
