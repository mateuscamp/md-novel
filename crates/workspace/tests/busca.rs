//! Buscar no livro inteiro (ADR 0003, "Buscar no livro"): sem diferenciar
//! maiúscula nem acento, com a posição que o editor usa, e só lendo.

use std::fs;

use workspace::{Achado, Livro, MAXIMO_DE_ACHADOS, buscar_no_texto};

/// O texto achado, como o editor o seleciona (posições em UTF-16).
fn selecionado(texto: &str, achado: &Achado) -> String {
    let unidades: Vec<u16> = texto.encode_utf16().collect();
    String::from_utf16(&unidades[achado.inicio..achado.fim]).expect("utf-16")
}

fn achados(texto: &str, termo: &str) -> Vec<String> {
    buscar_no_texto(texto, termo, usize::MAX)
        .iter()
        .map(|achado| selecionado(texto, achado))
        .collect()
}

#[test]
fn acha_sem_diferenciar_maiuscula_nem_acento() {
    let texto = "Coração e coracao.\nCORAÇÃO de mãe, e o pé no chão.";
    assert_eq!(achados(texto, "coração"), ["Coração", "coracao", "CORAÇÃO"]);
    assert_eq!(achados(texto, "CORACAO"), ["Coração", "coracao", "CORAÇÃO"]);
    assert_eq!(achados(texto, "pe"), ["pé"]);
    assert_eq!(achados(texto, "mae"), ["mãe"]);
    assert_eq!(achados("Maçã, maca e MAÇÃ", "ÇA"), ["çã", "ca", "ÇÃ"]);
    assert_eq!(achados(texto, "fantasma"), Vec::<String>::new());
}

#[test]
fn acento_escrito_em_dois_caracteres_tambem_e_achado_e_selecionado_inteiro() {
    // "pé" com o acento separado da letra, como alguns programas gravam.
    let texto = "o pe\u{301} e a mao\u{303}";
    assert_eq!(achados(texto, "pé"), ["pe\u{301}"]);
    assert_eq!(achados(texto, "mão"), ["mao\u{303}"]);
}

#[test]
fn a_posicao_e_a_do_editor_em_utf16() {
    // O emoji ocupa duas unidades em UTF-16, e o "é", uma.
    let texto = "😀 é casa";
    let achado = &buscar_no_texto(texto, "casa", usize::MAX)[0];
    assert_eq!((achado.inicio, achado.fim), (5, 9));
    // Vários achados com emoji entre eles: a conta segue de um para o outro.
    let texto = "😀 casa 😀 casa é 😀😀 casa";
    let lugares: Vec<(usize, usize)> = buscar_no_texto(texto, "CASA", usize::MAX)
        .iter()
        .map(|achado| (achado.inicio, achado.fim))
        .collect();
    assert_eq!(lugares, [(3, 7), (11, 15), (23, 27)]);
    assert_eq!(achados(texto, "casa"), ["casa", "casa", "casa"]);
}

#[test]
fn nao_se_sobrepoe_e_termo_vazio_nao_acha_nada() {
    assert_eq!(achados("aaaaa", "aa"), ["aa", "aa"]);
    // O texto tem espaços e quebras: só de espaços, o termo não acha nenhum.
    for vazio in ["", " ", "   ", "\n", "\t "] {
        assert!(
            buscar_no_texto("um texto\n\tcom   espaços \n", vazio, usize::MAX).is_empty(),
            "{vazio:?}"
        );
    }
    // Com letra, o espaço do termo conta.
    assert_eq!(achados("um texto, umtexto", "um texto"), ["um texto"]);
}

#[test]
fn o_trecho_mostra_a_linha_em_volta_cortada_com_reticencias() {
    let texto = format!(
        "Linha de cima.\n{}a cigana disse{}\nLinha de baixo.",
        "x".repeat(50),
        "y".repeat(80)
    );
    let achado = &buscar_no_texto(&texto, "cigana", usize::MAX)[0];
    assert_eq!(achado.trecho, "cigana");
    assert_eq!(achado.antes, format!("…{}a ", "x".repeat(38)));
    assert_eq!(achado.depois, format!(" disse{}…", "y".repeat(54)));
    // Perto da borda da linha não há reticências, nem texto da linha vizinha.
    let curto = &buscar_no_texto("Era uma vez.\nA cigana.\nFim.", "cigana", usize::MAX)[0];
    assert_eq!((curto.antes.as_str(), curto.depois.as_str()), ("A ", "."));
}

#[test]
fn o_limite_corta_a_busca() {
    let texto = "x ".repeat(10);
    assert_eq!(buscar_no_texto(&texto, "x", 3).len(), 3);
}

fn livro(capitulos: &[(&str, &[u8])]) -> tempfile::TempDir {
    let pasta = tempfile::tempdir().expect("pasta");
    for (nome, conteudo) in capitulos {
        fs::write(pasta.path().join(nome), conteudo).expect("escrever");
    }
    pasta
}

#[test]
fn busca_em_todos_os_capitulos_na_ordem_do_livro_so_lendo() {
    let capitulos: [(&str, &[u8]); 5] = [
        ("02.md", b"# Dois\r\n\r\nA cigana voltou.\r\n"),
        (
            "01.md",
            "# Um\n\nUma Cigana na ruela. Outra cigana.\n".as_bytes(),
        ),
        ("03.md", b"# Tres\n\nNada aqui.\n"),
        ("04.md", b"\xff\xfe cigana fora do UTF-8"),
        ("notas.txt", b"cigana fora do livro"),
    ];
    let pasta = livro(&capitulos);
    let busca = Livro::abrir(pasta.path())
        .expect("abrir")
        .buscar("CIGANA")
        .expect("buscar");
    assert!(!busca.cortada);
    let resumo: Vec<(&str, Option<&str>, usize)> = busca
        .capitulos
        .iter()
        .map(|capitulo| {
            (
                capitulo.nome.as_str(),
                capitulo.titulo.as_deref(),
                capitulo.achados.len(),
            )
        })
        .collect();
    // Só os capítulos com achado; o que não dá para ler fica de fora.
    assert_eq!(
        resumo,
        [("01.md", Some("Um"), 2), ("02.md", Some("Dois"), 1)]
    );
    // A posição é a do texto que o editor recebe: LF, sem o CR do arquivo.
    let no_dois = &busca.capitulos[1].achados[0];
    assert_eq!((no_dois.inicio, no_dois.fim), (10, 16));
    // Nada mudou na pasta.
    for (nome, conteudo) in capitulos {
        assert_eq!(
            fs::read(pasta.path().join(nome)).expect("ler"),
            conteudo,
            "{nome}"
        );
    }
    assert_eq!(fs::read_dir(pasta.path()).expect("listar").count(), 5);
}

#[test]
fn a_busca_no_livro_para_no_limite_e_diz_que_cortou() {
    let muitos = "x ".repeat(MAXIMO_DE_ACHADOS);
    let pasta = livro(&[("01.md", muitos.as_bytes()), ("02.md", b"x x x")]);
    let busca = Livro::abrir(pasta.path())
        .expect("abrir")
        .buscar("x")
        .expect("buscar");
    let total: usize = busca
        .capitulos
        .iter()
        .map(|capitulo| capitulo.achados.len())
        .sum();
    assert_eq!(total, MAXIMO_DE_ACHADOS);
    assert!(busca.cortada);
    // Exatamente no limite não é corte.
    let pasta = livro(&[("01.md", muitos.as_bytes())]);
    let busca = Livro::abrir(pasta.path())
        .expect("abrir")
        .buscar("x")
        .expect("buscar");
    assert!(!busca.cortada);
}
