//! O livro como pasta: listar, ler e salvar capítulos. O piso é a
//! integridade: abrir e salvar sem editar devolve os mesmos bytes, e o que
//! mudou por fora nunca é sobrescrito.

use std::fs;
use std::path::Path;

use workspace::{ErroWorkspace, Gravacao, Livro};

fn pasta_com(arquivos: &[(&str, &[u8])]) -> tempfile::TempDir {
    let pasta = tempfile::tempdir().expect("pasta temporária");
    for (nome, conteudo) in arquivos {
        fs::write(pasta.path().join(nome), conteudo).expect("criar arquivo");
    }
    pasta
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

#[test]
fn abrir_exige_uma_pasta_que_exista() {
    let pasta = pasta_com(&[("cap.md", b"texto")]);
    assert!(Livro::abrir(pasta.path()).is_ok());
    for ruim in [pasta.path().join("cap.md"), pasta.path().join("nao-existe")] {
        let erro = Livro::abrir(&ruim).expect_err("não é pasta");
        assert!(
            matches!(erro, ErroWorkspace::NaoEPasta { .. }),
            "veio {erro}"
        );
    }
}

#[test]
fn capitulos_sao_os_md_comuns_da_pasta_em_ordem_de_nome() {
    let pasta = pasta_com(&[
        ("10-fim.md", b"c"),
        ("02-meio.md", b"b"),
        ("01-abertura.md", b"a"),
        ("Notas.MD", b"maiuscula tambem conta"),
        ("estilo.yaml", b"versao: 1"),
        ("livro.docx", b"gerado"),
        (".oculto.md", b"nao entra"),
        (".md-novel-gravando-ab12CD.md", b"temporario nao entra"),
        ("sem-extensao", b"nao entra"),
    ]);
    fs::create_dir(pasta.path().join("pasta.md")).expect("pasta com nome de capítulo");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    assert_eq!(
        livro.capitulos().expect("listar"),
        ["01-abertura.md", "02-meio.md", "10-fim.md", "Notas.MD"]
    );
    assert_eq!(livro.caminho_do_estilo(), pasta.path().join("estilo.yaml"));
}

#[test]
fn abrir_o_livro_nao_cria_nem_muda_nada_na_pasta() {
    let pasta = pasta_com(&[("01.md", b"um"), ("estilo.yaml", b"versao: 1")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    livro.capitulos().expect("listar");
    livro.ler_capitulo("01.md").expect("ler");
    assert_eq!(nomes(pasta.path()), ["01.md", "estilo.yaml"]);
}

#[test]
fn so_le_capitulo_que_esta_na_lista() {
    let pasta = pasta_com(&[
        ("01.md", b"um"),
        ("estilo.yaml", b"versao: 1"),
        (".oculto.md", b"x"),
    ]);
    fs::write(
        pasta.path().parent().expect("pai").join("fora-do-livro.md"),
        b"fora",
    )
    .ok();
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    for nome in [
        "estilo.yaml",
        "../fora-do-livro.md",
        ".oculto.md",
        "nao-existe.md",
        "",
        "/etc/passwd",
    ] {
        let erro = livro.ler_capitulo(nome).expect_err("fora da lista");
        assert!(
            matches!(erro, ErroWorkspace::CapituloDesconhecido { .. }),
            "{nome:?}: veio {erro}"
        );
    }
}

/// O piso: para todo formato de arquivo que o editor aceita, ler e salvar o
/// mesmo texto não escreve nada, e os bytes do disco continuam os mesmos.
#[test]
fn abrir_e_salvar_sem_editar_devolve_os_mesmos_bytes() {
    let casos: [(&str, &[u8]); 8] = [
        ("lf", b"Um.\n\nDois.\n"),
        ("sem-linha-final", b"Um.\n\nDois."),
        ("crlf", b"Um.\r\n\r\nDois.\r\n"),
        ("bom", b"\xEF\xBB\xBFUm.\nDois.\n"),
        ("bom-crlf", b"\xEF\xBB\xBFUm.\r\nDois.\r\n"),
        ("espaco-final", b"Um.  \n\tDois. \n\n\n"),
        ("unicode", "Coração — “aspas” e emoji 🙂\n".as_bytes()),
        ("vazio", b""),
    ];
    for (nome, bytes) in casos {
        let pasta = pasta_com(&[("cap.md", bytes)]);
        let caminho = pasta.path().join("cap.md");
        let antes = fs::metadata(&caminho)
            .expect("metadados")
            .modified()
            .expect("data");
        let livro = Livro::abrir(pasta.path()).expect("abrir");
        let mut capitulo = livro
            .ler_capitulo("cap.md")
            .unwrap_or_else(|erro| panic!("{nome}: {erro}"));
        assert!(
            !capitulo.texto().contains('\r'),
            "{nome}: o editor recebe só LF"
        );
        assert!(
            !capitulo.texto().starts_with('\u{feff}'),
            "{nome}: o editor não recebe o BOM"
        );
        let texto = capitulo.texto().to_owned();
        assert_eq!(
            capitulo.salvar(&texto).expect("salvar"),
            Gravacao::SemMudanca,
            "{nome}"
        );
        assert_eq!(fs::read(&caminho).expect("reler"), bytes, "{nome}");
        assert_eq!(
            fs::metadata(&caminho)
                .expect("metadados")
                .modified()
                .expect("data"),
            antes,
            "{nome}: não regravou"
        );
        assert_eq!(nomes(pasta.path()), ["cap.md"], "{nome}");
    }
}

#[test]
fn salvar_uma_edicao_mantem_o_formato_do_arquivo() {
    let pasta = pasta_com(&[("cap.md", b"\xEF\xBB\xBFUm.\r\nDois.\r\n")]);
    let caminho = pasta.path().join("cap.md");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let mut capitulo = livro.ler_capitulo("cap.md").expect("ler");
    assert_eq!(capitulo.texto(), "Um.\nDois.\n");

    assert_eq!(
        capitulo.salvar("Um.\nDois.\nTrês.\n").expect("salvar"),
        Gravacao::Gravado
    );

    assert_eq!(
        fs::read(&caminho).expect("reler"),
        b"\xEF\xBB\xBFUm.\r\nDois.\r\nTr\xC3\xAAs.\r\n"
    );
    assert_eq!(capitulo.texto(), "Um.\nDois.\nTrês.\n");
    assert_eq!(nomes(pasta.path()), ["cap.md"], "sem temporário");
    // Salvar de novo o mesmo texto não escreve, e uma segunda edição passa:
    // o capítulo acompanha o que ele mesmo gravou.
    assert_eq!(
        capitulo.salvar("Um.\nDois.\nTrês.\n").expect("salvar"),
        Gravacao::SemMudanca
    );
    assert_eq!(capitulo.salvar("Um.\n").expect("salvar"), Gravacao::Gravado);
    assert_eq!(fs::read(&caminho).expect("reler"), b"\xEF\xBB\xBFUm.\r\n");
}

#[test]
fn arquivo_que_mudou_por_fora_nunca_e_sobrescrito() {
    let pasta = pasta_com(&[("cap.md", b"Um.\n")]);
    let caminho = pasta.path().join("cap.md");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let mut capitulo = livro.ler_capitulo("cap.md").expect("ler");
    fs::write(&caminho, b"Um, mudado em outro computador.\n").expect("mudança externa");

    let erro = capitulo
        .salvar("Um, editado aqui.\n")
        .expect_err("mudou por fora");

    assert!(
        matches!(erro, ErroWorkspace::MudouPorFora { .. }),
        "veio {erro}"
    );
    assert_eq!(
        fs::read(&caminho).expect("reler"),
        b"Um, mudado em outro computador.\n"
    );
    // Mesmo sem edição pendente, o capítulo aberto está velho: não confirma.
    let erro = capitulo.salvar("Um.\n").expect_err("continua velho");
    assert!(
        matches!(erro, ErroWorkspace::MudouPorFora { .. }),
        "veio {erro}"
    );
}

#[test]
fn arquivo_apagado_por_fora_nao_e_recriado_em_silencio() {
    let pasta = pasta_com(&[("cap.md", b"Um.\n")]);
    let caminho = pasta.path().join("cap.md");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let mut capitulo = livro.ler_capitulo("cap.md").expect("ler");
    fs::remove_file(&caminho).expect("apagar");

    let erro = capitulo
        .salvar("Um, editado.\n")
        .expect_err("apagado por fora");

    assert!(
        matches!(erro, ErroWorkspace::MudouPorFora { .. }),
        "veio {erro}"
    );
    assert!(!caminho.exists());
}

#[test]
fn arquivos_que_o_editor_nao_devolveria_iguais_sao_recusados_na_leitura() {
    let pasta = pasta_com(&[
        ("misturado.md", b"Um.\r\nDois.\nTr.\r\n"),
        ("cr-solto.md", b"Um.\rDois.\n"),
        ("latin1.md", b"Cora\xE7\xE3o\n"),
    ]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    for nome in ["misturado.md", "cr-solto.md"] {
        let erro = livro
            .ler_capitulo(nome)
            .expect_err("fim de linha misturado");
        assert!(
            matches!(erro, ErroWorkspace::FimDeLinhaMisturado { .. }),
            "{nome}: veio {erro}"
        );
    }
    let erro = livro.ler_capitulo("latin1.md").expect_err("não é UTF-8");
    assert!(
        matches!(erro, ErroWorkspace::NaoEUtf8 { .. }),
        "veio {erro}"
    );
}

#[test]
fn texto_colado_com_cr_e_gravado_no_formato_do_arquivo() {
    let pasta = pasta_com(&[("cap.md", b"Um.\n")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let mut capitulo = livro.ler_capitulo("cap.md").expect("ler");
    capitulo.salvar("Um.\r\nDois.\r\n").expect("salvar");
    assert_eq!(
        fs::read(pasta.path().join("cap.md")).expect("reler"),
        b"Um.\nDois.\n"
    );
}

#[test]
fn capitulo_acima_de_16_mib_e_recusado() {
    let pasta = pasta_com(&[("enorme.md", &vec![b'a'; 16 * 1024 * 1024 + 1])]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let erro = livro.ler_capitulo("enorme.md").expect_err("grande demais");
    assert!(
        matches!(erro, ErroWorkspace::CapituloGrandeDemais { .. }),
        "veio {erro}"
    );
}

#[cfg(unix)]
#[test]
fn salvar_mantem_a_permissao_do_capitulo() {
    use std::os::unix::fs::PermissionsExt;

    let pasta = pasta_com(&[("cap.md", b"Um.\n")]);
    let caminho = pasta.path().join("cap.md");
    fs::set_permissions(&caminho, fs::Permissions::from_mode(0o644)).expect("permissão");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let mut capitulo = livro.ler_capitulo("cap.md").expect("ler");
    capitulo.salvar("Um, editado.\n").expect("salvar");
    let modo = fs::metadata(&caminho)
        .expect("metadados")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(modo, 0o644, "veio {modo:o}");
}

#[test]
fn estilo_do_livro_e_lido_quando_existe_e_tem_teto_de_tamanho() {
    let sem = pasta_com(&[("01.md", b"um")]);
    assert_eq!(
        Livro::abrir(sem.path())
            .expect("abrir")
            .ler_estilo()
            .expect("ler"),
        None
    );

    let com = pasta_com(&[("estilo.yaml", b"versao: 1\n")]);
    assert_eq!(
        Livro::abrir(com.path())
            .expect("abrir")
            .ler_estilo()
            .expect("ler"),
        Some("versao: 1\n".to_owned())
    );

    let enorme = pasta_com(&[("estilo.yaml", &vec![b'#'; 1024 * 1024 + 1])]);
    let erro = Livro::abrir(enorme.path())
        .expect("abrir")
        .ler_estilo()
        .expect_err("grande demais");
    assert!(
        matches!(erro, ErroWorkspace::EstiloGrandeDemais { .. }),
        "veio {erro}"
    );

    let binario = pasta_com(&[("estilo.yaml", b"\xff\xfe\x00")]);
    let erro = Livro::abrir(binario.path())
        .expect("abrir")
        .ler_estilo()
        .expect_err("não é texto");
    assert!(
        matches!(erro, ErroWorkspace::NaoEUtf8 { .. }),
        "veio {erro}"
    );
}
