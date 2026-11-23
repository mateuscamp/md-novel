//! Regressões da exportação com o Pandoc de verdade (achados da auditoria de
//! 2026-11-23). Precisam do `pandoc` no PATH.

use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use exportar::{ErroExportar, exportar_docx, modelo_reference_docx};
use workspace::ErroWorkspace;
use zip::ZipArchive;

/// PNG válido de 1x1 pixel, para o Pandoc ter o que embutir.
const PNG_MINIMO: &[u8] = include_bytes!("fixtures/pixel.png");

/// Outro PNG válido, de 2x2, para distinguir duas imagens com o mesmo nome.
const PNG_2X2: &[u8] = include_bytes!("fixtures/pixel-2x2.png");

fn entradas(docx: &Path) -> ZipArchive<Cursor<Vec<u8>>> {
    ZipArchive::new(Cursor::new(fs::read(docx).expect("ler o .docx"))).expect("o .docx é um zip")
}

fn texto_da_entrada(docx: &Path, nome: &str) -> String {
    let mut texto = String::new();
    entradas(docx)
        .by_name(nome)
        .unwrap_or_else(|erro| panic!("falta {nome}: {erro}"))
        .read_to_string(&mut texto)
        .expect("entrada UTF-8");
    texto
}

fn modelo() -> Vec<u8> {
    modelo_reference_docx().expect("pedir o modelo ao pandoc")
}

#[test]
fn notas_com_o_mesmo_rotulo_em_capitulos_diferentes_nao_se_misturam() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let cap1 = pasta.path().join("cap1.md");
    let cap2 = pasta.path().join("cap2.md");
    fs::write(
        &cap1,
        "# Um\n\nTexto um.[^1]\n\n[^1]: Nota do capítulo um.\n",
    )
    .expect("cap1");
    fs::write(
        &cap2,
        "# Dois\n\nTexto dois.[^1]\n\n[^1]: Nota do capítulo dois.\n",
    )
    .expect("cap2");
    let saida = pasta.path().join("livro.docx");

    let avisos = exportar_docx(&[cap1, cap2], &modelo(), &saida).expect("exportar");

    let notas = texto_da_entrada(&saida, "word/footnotes.xml");
    assert!(notas.contains("Nota do capítulo um"), "{notas}");
    assert!(notas.contains("Nota do capítulo dois"), "{notas}");
    assert_eq!(
        avisos,
        Vec::<String>::new(),
        "exportação limpa não tem aviso"
    );
}

#[test]
fn imagem_ao_lado_do_capitulo_entra_mesmo_rodando_de_outra_pasta() {
    // O teste roda com a pasta atual no crate, e o livro fica em outro lugar.
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let livro = pasta.path().join("livro");
    fs::create_dir(&livro).expect("pasta do livro");
    fs::write(livro.join("mapa.png"), PNG_MINIMO).expect("imagem");
    let capitulo = livro.join("cap1.md");
    fs::write(&capitulo, "# Um\n\n![O mapa](mapa.png)\n").expect("capítulo");
    let saida = pasta.path().join("livro.docx");

    let avisos = exportar_docx(&[capitulo], &modelo(), &saida).expect("exportar");

    let imagens = entradas(&saida)
        .file_names()
        .filter_map(|nome| nome.ok().map(|nome| nome.into_owned()))
        .filter(|nome| nome.starts_with("word/media/"))
        .count();
    assert_eq!(
        imagens, 1,
        "a imagem do capítulo devia estar no .docx; avisos: {avisos:?}"
    );
    assert_eq!(avisos, Vec::<String>::new());
}

#[test]
fn aviso_do_pandoc_chega_a_quem_chamou() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let capitulo = pasta.path().join("cap1.md");
    fs::write(&capitulo, "# Um\n\n![Sumiu](nao-existe.png)\n").expect("capítulo");
    let saida = pasta.path().join("livro.docx");

    let avisos = exportar_docx(&[capitulo], &modelo(), &saida).expect("aviso não é erro");

    assert!(saida.exists(), "o .docx sai mesmo com aviso");
    assert!(
        avisos.iter().any(|aviso| aviso.contains("nao-existe.png")),
        "esperava o aviso da imagem que falta, veio {avisos:?}"
    );
}

#[test]
fn pasta_de_saida_que_nao_existe_e_erro_claro() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let capitulo = pasta.path().join("cap1.md");
    fs::write(&capitulo, "texto").expect("capítulo");
    let saida = pasta.path().join("nao-existe").join("livro.docx");

    let erro = exportar_docx(&[capitulo], b"", &saida).expect_err("pasta inexistente");

    assert!(
        matches!(
            erro,
            ErroExportar::Saida(ErroWorkspace::PastaInexistente { .. })
        ),
        "veio {erro}"
    );
    let mensagem = erro.to_string();
    assert!(mensagem.contains("nao-existe"), "{mensagem}");
    assert!(
        !mensagem.contains(".md-novel-"),
        "a mensagem não cita o temporário: {mensagem}"
    );
}

#[cfg(unix)]
#[test]
fn substituir_um_docx_existente_mantem_a_permissao_dele() {
    use std::os::unix::fs::PermissionsExt;

    let pasta = tempfile::tempdir().expect("pasta temporária");
    let capitulo = pasta.path().join("cap1.md");
    fs::write(&capitulo, "# Um\n\nTexto.\n").expect("capítulo");
    let saida = pasta.path().join("livro.docx");
    fs::write(&saida, "versão antiga").expect("docx antigo");
    fs::set_permissions(&saida, fs::Permissions::from_mode(0o644)).expect("permissão");

    exportar_docx(&[capitulo], &modelo(), &saida).expect("exportar");

    let modo = fs::metadata(&saida)
        .expect("metadados")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(modo, 0o644, "veio {modo:o}");
    assert!(texto_da_entrada(&saida, "word/document.xml").contains("Texto."));
}

// Regressões da segunda auditoria de 2026-11-23.

#[test]
fn imagens_com_o_mesmo_nome_em_pastas_diferentes_nao_se_trocam() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    // O nome com dois-pontos confere que nenhuma lista de pastas é montada.
    let (parte1, parte2) = (
        pasta.path().join("parte1"),
        pasta.path().join("parte 2: fim"),
    );
    fs::create_dir(&parte1).expect("parte1");
    fs::create_dir(&parte2).expect("parte2");
    fs::write(parte1.join("mapa.png"), PNG_MINIMO).expect("mapa um");
    fs::write(parte2.join("mapa.png"), PNG_2X2).expect("mapa dois");
    fs::write(parte1.join("cap1.md"), "# Um\n\n![Mapa um](mapa.png)\n").expect("cap1");
    fs::write(parte2.join("cap2.md"), "# Dois\n\n![Mapa dois](mapa.png)\n").expect("cap2");
    let saida = pasta.path().join("livro.docx");

    let avisos = exportar_docx(
        &[parte1.join("cap1.md"), parte2.join("cap2.md")],
        &modelo(),
        &saida,
    )
    .expect("exportar");

    let mut zip = entradas(&saida);
    let nomes: Vec<String> = zip
        .file_names()
        .filter_map(|nome| nome.ok().map(|nome| nome.into_owned()))
        .filter(|nome| nome.starts_with("word/media/"))
        .collect();
    let mut imagens: Vec<Vec<u8>> = Vec::new();
    for nome in &nomes {
        let mut bytes = Vec::new();
        zip.by_name(nome)
            .expect("imagem")
            .read_to_end(&mut bytes)
            .expect("ler imagem");
        imagens.push(bytes);
    }
    assert_eq!(
        imagens.len(),
        2,
        "cada capítulo tem a sua imagem; avisos: {avisos:?}"
    );
    assert!(
        imagens.iter().any(|imagem| imagem == PNG_MINIMO),
        "falta o mapa da parte 1"
    );
    assert!(
        imagens.iter().any(|imagem| imagem == PNG_2X2),
        "falta o mapa da parte 2"
    );
    assert_eq!(avisos, Vec::<String>::new());
}

#[test]
fn capitulo_que_nao_existe_e_erro_do_projeto_e_nao_do_pandoc() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let existe = pasta.path().join("cap1.md");
    fs::write(&existe, "texto").expect("cap1");
    let falta = pasta.path().join("cap9.md");
    let saida = pasta.path().join("livro.docx");

    let erro = exportar_docx(&[existe, falta], b"", &saida).expect_err("capítulo que falta");

    assert!(
        matches!(erro, ErroExportar::CapituloInexistente { .. }),
        "veio {erro}"
    );
    let mensagem = erro.to_string();
    assert!(mensagem.contains("cap9.md"), "{mensagem}");
    assert!(!mensagem.contains("pandoc"), "{mensagem}");
    assert!(!saida.exists(), "nada é gravado quando falta um capítulo");
}

// Regressão da terceira auditoria de 2026-11-23.

#[test]
fn pasta_passada_como_capitulo_diz_que_nao_e_arquivo() {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    let capitulos = pasta.path().join("capitulos");
    fs::create_dir(&capitulos).expect("pasta de capítulos");
    let saida = pasta.path().join("livro.docx");

    let erro = exportar_docx(std::slice::from_ref(&capitulos), b"", &saida).expect_err("pasta");

    assert!(
        matches!(erro, ErroExportar::CapituloNaoEArquivo { .. }),
        "veio {erro}"
    );
    let mensagem = erro.to_string();
    assert!(
        mensagem.contains("capitulos") && mensagem.contains("pasta"),
        "{mensagem}"
    );
    assert!(
        !mensagem.contains("não achei"),
        "a pasta existe: {mensagem}"
    );
}
