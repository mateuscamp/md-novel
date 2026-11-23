//! Fitness functions de fronteira (ADR 0003): o núcleo não conhece a janela.
//! `workspace` é Rust puro, e nenhum crate do núcleo depende de tauri.

use std::fs;
use std::path::Path;

const DEPENDENCIAS_PERMITIDAS: [&str; 2] = ["tempfile", "thiserror"];
const PROIBIDO_NO_CODIGO: [&str; 4] = ["std::process", "std::net", "Command::new", "tauri"];
const CRATES_DO_NUCLEO: [&str; 4] = ["estilo", "workspace", "exportar", "cli"];

fn dependencias(manifesto: &str) -> Vec<String> {
    manifesto
        .split("\n[")
        .filter(|secao| secao.starts_with("dependencies]") || secao.contains("dependencies]\n"))
        .flat_map(|secao| secao.lines().skip(1))
        .filter_map(|linha| linha.split(['.', '=', ' ']).next())
        .filter(|nome| !nome.is_empty() && !nome.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn workspace_so_tem_dependencias_permitidas() {
    let manifesto = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("ler o Cargo.toml do crate");
    let encontradas = dependencias(&manifesto);
    assert!(
        !encontradas.is_empty(),
        "não achei dependências em:\n{manifesto}"
    );
    for nome in encontradas {
        assert!(
            DEPENDENCIAS_PERMITIDAS.contains(&nome.as_str()),
            "dependência {nome:?} não é permitida em crates/workspace; registre num ADR antes"
        );
    }
}

#[test]
fn workspace_nao_abre_processo_nem_rede_nem_conhece_a_janela() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut lidos = 0;
    for arquivo in fs::read_dir(&src).expect("listar src") {
        let caminho = arquivo.expect("entrada de src").path();
        let codigo = fs::read_to_string(&caminho).expect("ler fonte");
        lidos += 1;
        // Só o código: os comentários podem citar o app.
        let sem_comentarios: String = codigo
            .lines()
            .filter(|linha| !linha.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for proibido in PROIBIDO_NO_CODIGO {
            assert!(
                !sem_comentarios.contains(proibido),
                "{} usa {proibido}; isso não entra em crates/workspace",
                caminho.display()
            );
        }
    }
    assert!(lidos >= 4, "esperava ler os módulos de src, li {lidos}");
}

#[test]
fn nenhum_crate_do_nucleo_depende_de_tauri() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for nome in CRATES_DO_NUCLEO {
        let manifesto = fs::read_to_string(crates.join(nome).join("Cargo.toml"))
            .unwrap_or_else(|erro| panic!("ler o Cargo.toml de {nome}: {erro}"));
        let com_tauri: Vec<String> = dependencias(&manifesto)
            .into_iter()
            .filter(|dependencia| dependencia.contains("tauri"))
            .collect();
        assert!(
            com_tauri.is_empty(),
            "crates/{nome} depende de {com_tauri:?}; a janela fica só em app/src-tauri (ADR 0003)"
        );
    }
}
