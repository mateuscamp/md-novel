//! O livro como pasta: listar, ler e salvar capítulos. O piso é a
//! integridade: abrir e salvar sem editar devolve os mesmos bytes, e o que
//! mudou por fora nunca é sobrescrito.

use std::fs;
use std::path::Path;

use workspace::{ErroWorkspace, Gravacao, Livro, criar_livro};

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

#[test]
fn resumos_trazem_titulo_e_palavras_de_cada_capitulo_na_ordem() {
    let pasta = pasta_com(&[
        (
            "02-a-casa.md",
            "# A casa\n\nEla abria as janelas.\n".as_bytes(),
        ),
        ("01-abertura.md", b"Sem titulo, cinco palavras aqui.\n"),
        ("03-quebrado.md", b"Cora\xE7\xE3o\n"),
    ]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");

    let resumos = livro.resumos().expect("resumos");

    let visto: Vec<(&str, Option<&str>, Option<usize>)> = resumos
        .iter()
        .map(|resumo| {
            (
                resumo.nome.as_str(),
                resumo.titulo.as_deref(),
                resumo.palavras,
            )
        })
        .collect();
    assert_eq!(
        visto,
        [
            ("01-abertura.md", None, Some(5)),
            ("02-a-casa.md", Some("A casa"), Some(6)),
            // Capítulo que não dá para ler continua na lista, sem números:
            // o erro aparece quando o autor tenta abri-lo.
            ("03-quebrado.md", None, None),
        ]
    );
}

// Criar o estilo.yaml é a única gravação do app na pasta do livro que não é
// salvar um capítulo (ADR 0003, "Criar o estilo.yaml pelo app"). O autor
// pede pelo botão; o que já existe nunca é trocado.
const MODELO: &str = "versao: 1\ncorpo:\n  fonte: EB Garamond\n";

#[test]
fn criar_estilo_grava_o_texto_em_estilo_yaml_e_mais_nada() {
    let pasta = pasta_com(&[("01.md", b"um")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");

    livro.criar_estilo(MODELO).expect("criar o estilo");

    assert_eq!(
        fs::read_to_string(pasta.path().join("estilo.yaml")).expect("ler o estilo"),
        MODELO
    );
    // Sem temporário esquecido, e o capítulo intacto.
    assert_eq!(nomes(pasta.path()), ["01.md", "estilo.yaml"]);
    assert_eq!(livro.ler_estilo().expect("ler").as_deref(), Some(MODELO));
}

#[test]
fn criar_estilo_nunca_grava_por_cima_de_um_que_ja_existe() {
    let do_autor = b"versao: 1\ncorpo:\n  fonte: A do autor\n";
    let pasta = pasta_com(&[("01.md", b"um"), ("estilo.yaml", do_autor)]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");

    let erro = livro.criar_estilo(MODELO).expect_err("já existe");

    assert!(
        matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with("estilo.yaml")),
        "{erro:?}"
    );
    let mensagem = erro.to_string();
    assert!(mensagem.contains("estilo.yaml"), "{mensagem}");
    assert!(mensagem.contains("nada foi gravado"), "{mensagem}");
    assert_eq!(
        fs::read(pasta.path().join("estilo.yaml")).expect("ler"),
        do_autor
    );
    assert_eq!(nomes(pasta.path()), ["01.md", "estilo.yaml"]);
}

#[cfg(unix)]
#[test]
fn criar_estilo_nao_segue_link_nem_troca_o_link() {
    // Um estilo.yaml que é link para outro arquivo já é um estilo: o do
    // autor, guardado em outro lugar. Quebrado, também não vira arquivo novo.
    let pasta = pasta_com(&[("01.md", b"um"), ("guardado.yaml", b"do autor")]);
    std::os::unix::fs::symlink("guardado.yaml", pasta.path().join("estilo.yaml"))
        .expect("criar o link");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let ja_existe = |erro: ErroWorkspace| matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with("estilo.yaml"));
    assert!(ja_existe(
        livro
            .criar_estilo(MODELO)
            .expect_err("o link já é o estilo")
    ));
    assert_eq!(
        fs::read(pasta.path().join("guardado.yaml")).expect("ler"),
        b"do autor"
    );
    assert!(
        fs::symlink_metadata(pasta.path().join("estilo.yaml"))
            .expect("metadados")
            .file_type()
            .is_symlink()
    );

    fs::remove_file(pasta.path().join("guardado.yaml")).expect("quebrar o link");
    assert!(ja_existe(
        livro
            .criar_estilo(MODELO)
            .expect_err("o link quebrado também")
    ));
    assert_eq!(nomes(pasta.path()), ["01.md", "estilo.yaml"]);
}

// Criar um capítulo pelo app (ADR 0003, "Capítulo novo pelo app"): o autor dá
// o nome, e o arquivo nasce vazio. O que já existe nunca é trocado.
#[test]
fn criar_capitulo_cria_um_md_vazio_com_o_nome_dado_e_ele_entra_na_lista() {
    let pasta = pasta_com(&[("01-abertura.md", b"um")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");

    assert_eq!(
        livro.criar_capitulo("02-a-casa").expect("criar"),
        "02-a-casa.md"
    );
    // Com a extensão digitada, com espaços em volta e com acento: vale igual.
    assert_eq!(
        livro.criar_capitulo("  03-o-rio.MD ").expect("criar"),
        "03-o-rio.md"
    );
    assert_eq!(
        livro.criar_capitulo("04 São João").expect("criar"),
        "04 São João.md"
    );

    assert_eq!(
        fs::read(pasta.path().join("02-a-casa.md")).expect("ler"),
        b""
    );
    assert_eq!(
        nomes(pasta.path()),
        [
            "01-abertura.md",
            "02-a-casa.md",
            "03-o-rio.md",
            "04 São João.md"
        ]
    );
    assert_eq!(
        livro.capitulos().expect("listar"),
        nomes(pasta.path()),
        "o capítulo novo é um capítulo como os outros"
    );
    // E abre e salva como qualquer capítulo.
    let mut novo = livro.ler_capitulo("02-a-casa.md").expect("ler o novo");
    assert_eq!(
        novo.salvar("# A casa\n").expect("salvar"),
        Gravacao::Gravado
    );
}

#[test]
fn criar_capitulo_nunca_grava_por_cima_do_que_ja_existe() {
    let pasta = pasta_com(&[("01.md", b"texto do autor")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    for nome in ["01", "01.md"] {
        let erro = livro.criar_capitulo(nome).expect_err("já existe");
        assert!(
            matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with("01.md")),
            "{erro:?}"
        );
    }
    assert_eq!(
        fs::read(pasta.path().join("01.md")).expect("ler"),
        b"texto do autor"
    );
    assert_eq!(nomes(pasta.path()), ["01.md"]);
}

#[test]
fn nome_de_capitulo_que_sai_da_pasta_ou_nao_serve_de_arquivo_e_recusado() {
    let base = pasta_com(&[]);
    fs::create_dir(base.path().join("livro")).expect("criar a pasta do livro");
    let livro = Livro::abrir(&base.path().join("livro")).expect("abrir");
    let longo = "a".repeat(101);
    let recusados = [
        "",
        "   ",
        ".md",
        "../fora",
        "sub/capitulo",
        "sub\\capitulo",
        "..",
        ".oculto",
        ".md-novel-gravando-x",
        "com\nquebra",
        "com\ttabulacao",
        "dois:pontos",
        "per?gunta",
        "aste*risco",
        "as\"pas",
        "me<nor",
        "bar|ra",
        "fim com ponto.",
        longo.as_str(),
    ];
    for nome in recusados {
        let erro = livro.criar_capitulo(nome).expect_err(nome);
        assert!(
            matches!(erro, ErroWorkspace::NomeDeCapituloInvalido { .. }),
            "{nome:?}: {erro:?}"
        );
        let mensagem = erro.to_string();
        assert!(
            mensagem.contains("03-a-casa"),
            "falta o exemplo: {mensagem}"
        );
    }
    // Nada dentro do livro, nada fora dele.
    assert_eq!(nomes(&base.path().join("livro")), Vec::<String>::new());
    assert_eq!(nomes(base.path()), ["livro"]);
}

// O painel "Estilo" do app grava o estilo.yaml (ADR 0003). Como no capítulo,
// só grava se o arquivo está como o painel o leu: o que mudou por fora nunca
// é trocado.
#[test]
fn gravar_estilo_cria_o_arquivo_que_nao_existia_e_troca_o_que_o_painel_leu() {
    let pasta = pasta_com(&[("01.md", b"um")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");

    livro.gravar_estilo("primeiro\n", None).expect("criar");
    assert_eq!(
        livro.ler_estilo().expect("ler").as_deref(),
        Some("primeiro\n")
    );
    livro
        .gravar_estilo("segundo\n", Some("primeiro\n"))
        .expect("trocar");
    assert_eq!(
        livro.ler_estilo().expect("ler").as_deref(),
        Some("segundo\n")
    );
    // Gravar o que já está lá não é erro nem deixa temporário.
    livro
        .gravar_estilo("segundo\n", Some("segundo\n"))
        .expect("igual");
    assert_eq!(nomes(pasta.path()), ["01.md", "estilo.yaml"]);
}

#[test]
fn gravar_estilo_recusa_quando_o_arquivo_nao_esta_como_o_painel_leu() {
    let do_autor = "editado à mão\n";
    let pasta = pasta_com(&[("01.md", b"um"), ("estilo.yaml", do_autor.as_bytes())]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    // O painel leu outra coisa, ou leu "não existe": o arquivo mudou por fora.
    for lido in [Some("o que o painel leu\n"), None] {
        let erro = livro.gravar_estilo("do painel\n", lido).expect_err("mudou");
        assert!(
            matches!(erro, ErroWorkspace::EstiloMudouPorFora { .. }),
            "{erro:?}"
        );
        assert!(erro.to_string().contains("estilo.yaml"), "{erro}");
        assert!(erro.to_string().contains("nada foi gravado"), "{erro}");
    }
    assert_eq!(livro.ler_estilo().expect("ler").as_deref(), Some(do_autor));

    // E o contrário: o painel leu um arquivo que foi apagado por fora.
    fs::remove_file(pasta.path().join("estilo.yaml")).expect("apagar");
    let erro = livro
        .gravar_estilo("do painel\n", Some(do_autor))
        .expect_err("sumiu");
    assert!(
        matches!(erro, ErroWorkspace::EstiloMudouPorFora { .. }),
        "{erro:?}"
    );
    assert_eq!(nomes(pasta.path()), ["01.md"]);
}

// Renomear capítulo e livro, e criar livro (ADR 0003, "Vários livros"). O
// livro é a pasta: renomear muda um nome e nada mais, e nunca passa por cima
// do que já existe, nem de uma pasta vazia, nem de um link quebrado.

#[test]
fn renomear_capitulo_muda_so_o_nome_do_arquivo() {
    // BOM e CR LF: os bytes do capítulo precisam sair exatamente iguais.
    let bytes: &[u8] = b"\xEF\xBB\xBF# Um\r\n\r\nTexto.\r\n";
    let pasta = pasta_com(&[("01.md", bytes), ("02.md", b"dois")]);
    let livro = Livro::abrir(pasta.path()).expect("abrir");

    assert_eq!(
        livro
            .renomear_capitulo("01.md", " 01 A partida ")
            .expect("renomear"),
        "01 A partida.md"
    );
    assert_eq!(nomes(pasta.path()), ["01 A partida.md", "02.md"]);
    assert_eq!(
        fs::read(pasta.path().join("01 A partida.md")).expect("ler"),
        bytes
    );
    assert_eq!(livro.capitulos().expect("listar"), nomes(pasta.path()));
    // Com o mesmo nome, digitado com ou sem .md, nada muda.
    for igual in ["02", "02.md"] {
        assert_eq!(
            livro.renomear_capitulo("02.md", igual).expect("mesmo nome"),
            "02.md"
        );
    }
    assert_eq!(nomes(pasta.path()), ["01 A partida.md", "02.md"]);
}

#[test]
fn renomear_capitulo_nunca_passa_por_cima_de_outro() {
    let pasta = pasta_com(&[("01.md", b"um"), ("02.md", b"dois")]);
    // Uma pasta com nome de capítulo não é capítulo, mas ocupa o nome.
    fs::create_dir(pasta.path().join("pasta.md")).expect("criar pasta");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    for (novo, ocupado) in [("02", "02.md"), ("pasta", "pasta.md")] {
        let erro = livro.renomear_capitulo("01.md", novo).expect_err(novo);
        assert!(
            matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with(ocupado)),
            "{novo}: {erro:?}"
        );
    }
    assert_eq!(fs::read(pasta.path().join("01.md")).expect("ler"), b"um");
    assert_eq!(fs::read(pasta.path().join("02.md")).expect("ler"), b"dois");
    assert!(pasta.path().join("pasta.md").is_dir());
    assert_eq!(nomes(pasta.path()), ["01.md", "02.md", "pasta.md"]);
}

#[cfg(unix)]
#[test]
fn renomear_capitulo_nao_passa_por_cima_de_link_quebrado() {
    let pasta = pasta_com(&[("01.md", b"um")]);
    std::os::unix::fs::symlink("nao-existe.md", pasta.path().join("link.md")).expect("criar link");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let erro = livro.renomear_capitulo("01.md", "link").expect_err("link");
    assert!(matches!(erro, ErroWorkspace::JaExiste { .. }), "{erro:?}");
    assert_eq!(fs::read(pasta.path().join("01.md")).expect("ler"), b"um");
    assert!(
        fs::symlink_metadata(pasta.path().join("link.md"))
            .expect("o link continua")
            .is_symlink()
    );
}

#[test]
fn renomear_capitulo_so_aceita_capitulo_da_lista_e_nome_que_serve() {
    let base = pasta_com(&[("fora.md", b"fora")]);
    let pasta = base.path().join("livro");
    fs::create_dir_all(pasta.join("sub")).expect("criar pastas");
    for (nome, conteudo) in [
        ("01.md", "um"),
        (".oculto.md", "oculto"),
        ("notas.txt", "notas"),
        ("sub/x.md", "x"),
    ] {
        fs::write(pasta.join(nome), conteudo).expect("criar arquivo");
    }
    let livro = Livro::abrir(&pasta).expect("abrir");
    for atual in [
        "nao-existe.md",
        "../fora.md",
        "sub/x.md",
        ".oculto.md",
        "notas.txt",
    ] {
        let erro = livro.renomear_capitulo(atual, "novo").expect_err(atual);
        assert!(
            matches!(erro, ErroWorkspace::CapituloDesconhecido { .. }),
            "{atual}: {erro:?}"
        );
    }
    for novo in ["", "  ", "../fora", "sub/x", "..", ".oculto", "dois:pontos"] {
        let erro = livro.renomear_capitulo("01.md", novo).expect_err(novo);
        assert!(
            matches!(erro, ErroWorkspace::NomeDeCapituloInvalido { .. }),
            "{novo:?}: {erro:?}"
        );
    }
    assert_eq!(nomes(&pasta), [".oculto.md", "01.md", "notas.txt", "sub"]);
    assert_eq!(nomes(&pasta.join("sub")), ["x.md"]);
    assert_eq!(nomes(base.path()), ["fora.md", "livro"]);
}

#[test]
fn renomear_o_livro_muda_o_nome_da_pasta_e_nada_dentro_dela() {
    let base = pasta_com(&[("O dom da pele.docx", b"docx do autor")]);
    let pasta = base.path().join("o-dom-da-pele-md");
    fs::create_dir_all(pasta.join("imagens")).expect("criar pastas");
    for (nome, conteudo) in [
        ("01.md", "# Um\n"),
        ("estilo.yaml", "versao: 1\n"),
        ("imagens/a.png", "png"),
    ] {
        fs::write(pasta.join(nome), conteudo).expect("criar arquivo");
    }
    let livro = Livro::abrir(&pasta).expect("abrir");

    let renomeado = livro.renomear(" O dom da pele ").expect("renomear");
    assert_eq!(renomeado.pasta(), base.path().join("O dom da pele"));
    assert_eq!(nomes(base.path()), ["O dom da pele", "O dom da pele.docx"]);
    assert_eq!(
        nomes(renomeado.pasta()),
        ["01.md", "estilo.yaml", "imagens"]
    );
    assert_eq!(
        fs::read(base.path().join("O dom da pele.docx")).expect("ler"),
        b"docx do autor"
    );
    assert_eq!(
        fs::read(renomeado.pasta().join("imagens/a.png")).expect("ler"),
        b"png"
    );
    // O livro renomeado é um livro como antes: lê e salva no lugar novo.
    let mut capitulo = renomeado.ler_capitulo("01.md").expect("ler");
    assert_eq!(
        capitulo.salvar("# Um\n\nMais.\n").expect("salvar"),
        Gravacao::Gravado
    );
    assert_eq!(
        fs::read(renomeado.pasta().join("01.md")).expect("ler"),
        b"# Um\n\nMais.\n"
    );
    // O mesmo nome não muda nada.
    let igual = renomeado.renomear("O dom da pele").expect("mesmo nome");
    assert_eq!(igual.pasta(), renomeado.pasta());
    assert_eq!(nomes(base.path()), ["O dom da pele", "O dom da pele.docx"]);
}

#[test]
fn renomear_o_livro_nunca_passa_por_cima_de_outra_pasta_nem_de_arquivo() {
    let base = pasta_com(&[("arquivo", b"do autor")]);
    let pasta = base.path().join("livro");
    fs::create_dir(&pasta).expect("criar livro");
    fs::write(pasta.join("01.md"), "um").expect("criar capítulo");
    // A pasta vazia é a que o sistema trocaria sem erro num rename comum.
    fs::create_dir(base.path().join("vazia")).expect("criar pasta vazia");
    let livro = Livro::abrir(&pasta).expect("abrir");
    for ocupado in ["vazia", "arquivo"] {
        let erro = livro.renomear(ocupado).expect_err(ocupado);
        assert!(
            matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with(ocupado)),
            "{ocupado}: {erro:?}"
        );
    }
    assert_eq!(nomes(base.path()), ["arquivo", "livro", "vazia"]);
    assert_eq!(nomes(&pasta), ["01.md"]);
    assert_eq!(nomes(&base.path().join("vazia")), Vec::<String>::new());
    assert_eq!(
        fs::read(base.path().join("arquivo")).expect("ler"),
        b"do autor"
    );
}

#[cfg(unix)]
#[test]
fn renomear_o_livro_nao_passa_por_cima_de_link_quebrado() {
    let base = pasta_com(&[]);
    let pasta = base.path().join("livro");
    fs::create_dir(&pasta).expect("criar livro");
    std::os::unix::fs::symlink("nao-existe", base.path().join("link")).expect("criar link");
    let erro = Livro::abrir(&pasta)
        .expect("abrir")
        .renomear("link")
        .expect_err("link");
    assert!(matches!(erro, ErroWorkspace::JaExiste { .. }), "{erro:?}");
    assert!(pasta.is_dir());
    assert!(
        fs::symlink_metadata(base.path().join("link"))
            .expect("o link continua")
            .is_symlink()
    );
}

#[test]
fn nome_de_livro_que_sai_da_pasta_ou_nao_serve_e_recusado() {
    let base = pasta_com(&[]);
    let pasta = base.path().join("livro");
    fs::create_dir(&pasta).expect("criar livro");
    let livro = Livro::abrir(&pasta).expect("abrir");
    let longo = "a".repeat(101);
    let recusados = [
        "",
        "   ",
        ".",
        "..",
        "../fora",
        "a/b",
        "a\\b",
        ".oculto",
        "dois:pontos",
        "per?gunta",
        "fim com ponto.",
        "com\nquebra",
        longo.as_str(),
    ];
    for nome in recusados {
        for erro in [
            livro.renomear(nome).expect_err(nome),
            criar_livro(base.path(), nome).expect_err(nome),
        ] {
            assert!(
                matches!(erro, ErroWorkspace::NomeDeLivroInvalido { .. }),
                "{nome:?}: {erro:?}"
            );
            assert!(
                erro.to_string().contains("O dom da pele"),
                "falta o exemplo: {erro}"
            );
        }
    }
    assert_eq!(nomes(base.path()), ["livro"]);
}

#[test]
fn criar_livro_cria_uma_pasta_vazia_e_nunca_reaproveita_o_que_existe() {
    let base = pasta_com(&[("arquivo", b"do autor")]);
    let novo = criar_livro(base.path(), " Livro Novo ").expect("criar");
    assert_eq!(novo.pasta(), base.path().join("Livro Novo"));
    assert_eq!(nomes(novo.pasta()), Vec::<String>::new());
    assert_eq!(novo.capitulos().expect("listar"), Vec::<String>::new());

    for ocupado in ["Livro Novo", "arquivo"] {
        let erro = criar_livro(base.path(), ocupado).expect_err(ocupado);
        assert!(
            matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with(ocupado)),
            "{ocupado}: {erro:?}"
        );
    }
    let erro = criar_livro(&base.path().join("nao-existe"), "x").expect_err("sem pasta");
    assert!(matches!(erro, ErroWorkspace::NaoEPasta { .. }), "{erro:?}");
    assert_eq!(nomes(base.path()), ["Livro Novo", "arquivo"]);
    assert_eq!(
        fs::read(base.path().join("arquivo")).expect("ler"),
        b"do autor"
    );
}

#[test]
fn renomear_trocando_so_maiusculas_funciona_ate_no_disco_que_nao_as_distingue() {
    // O disco do Mac, por padrão, acha "a-casa" e "A-Casa" o mesmo nome: a
    // troca que recusa destino existente recusaria sem esta exceção.
    let base = pasta_com(&[]);
    let pasta = base.path().join("livro");
    fs::create_dir(&pasta).expect("criar livro");
    fs::write(pasta.join("01-a-casa.md"), "casa").expect("criar capítulo");
    let livro = Livro::abrir(&pasta).expect("abrir");
    assert_eq!(
        livro
            .renomear_capitulo("01-a-casa.md", "01-A-Casa")
            .expect("renomear capítulo"),
        "01-A-Casa.md"
    );
    assert_eq!(nomes(&pasta), ["01-A-Casa.md"]);
    let renomeado = livro.renomear("Livro").expect("renomear livro");
    assert_eq!(renomeado.pasta(), base.path().join("Livro"));
    assert_eq!(nomes(base.path()), ["Livro"]);
    assert_eq!(
        fs::read(base.path().join("Livro/01-A-Casa.md")).expect("ler"),
        b"casa"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn trocar_a_caixa_para_o_nome_de_outro_link_do_mesmo_arquivo_e_recusado() {
    // No Linux, `a.md` e `A.md` podem ser dois nomes do mesmo arquivo. O
    // sistema não faz nada ao renomear um por cima do outro: dizer que deu
    // certo seria mentir.
    let pasta = pasta_com(&[("a.md", b"texto")]);
    fs::hard_link(pasta.path().join("a.md"), pasta.path().join("A.md")).expect("criar link");
    let livro = Livro::abrir(pasta.path()).expect("abrir");
    let erro = livro.renomear_capitulo("a.md", "A").expect_err("já existe");
    assert!(
        matches!(erro, ErroWorkspace::JaExiste { ref recebido } if recebido.ends_with("A.md")),
        "{erro:?}"
    );
    assert_eq!(nomes(pasta.path()), ["A.md", "a.md"]);
}
