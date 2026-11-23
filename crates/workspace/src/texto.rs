//! O que se lê do texto de um capítulo sem interpretar o Markdown inteiro:
//! quantas palavras tem e qual é o título. A interface tem as mesmas duas
//! funções (`app/src/texto.ts`), para contar ao vivo; os casos de
//! `tests/fixtures` são lidos pelos dois lados e mantêm as regras iguais.

/// Conta as palavras de um capítulo.
///
/// Palavra é todo trecho entre espaços que tenha ao menos uma letra ou um
/// número: `—`, `#` e `* * *` não contam, `d'água` e `texto[^1]` contam uma
/// vez. As linhas que abrem e fecham um bloco `:::` são marcação, não texto.
///
/// ```
/// assert_eq!(workspace::contar_palavras("# Um\n\nA casa — enfim."), 4);
/// ```
pub fn contar_palavras(texto: &str) -> usize {
    texto
        .lines()
        .filter(|linha| !linha.trim_start().starts_with(":::"))
        .flat_map(str::split_whitespace)
        .filter(|trecho| trecho.chars().any(char::is_alphanumeric))
        .count()
}

/// O título do capítulo: o texto do primeiro título Markdown (`# ...`) que
/// tenha texto. `None` se o capítulo não tem.
///
/// Título aqui é o que o **Pandoc** lê como título, porque é ele quem escreve
/// o .docx (`crates/exportar/tests/titulos_pandoc.rs` confere os casos com o
/// Pandoc de verdade): fora de bloco de código, na primeira coluna, e nunca
/// colado num parágrafo. Sem linha em branco antes, o `# ...` é texto do
/// parágrafo de cima.
///
/// ```
/// assert_eq!(workspace::titulo_do_texto("# A casa\n\nTexto."), Some("A casa".to_owned()));
/// assert_eq!(workspace::titulo_do_texto("Texto.\n# Colado"), None);
/// ```
pub fn titulo_do_texto(texto: &str) -> Option<String> {
    // A cerca (crases ou til) que abriu o bloco de código em que estamos.
    let mut cerca: Option<&str> = None;
    // Quantos blocos `:::` estão abertos.
    let mut blocos_abertos = 0_usize;
    // A linha de cima é texto de parágrafo.
    let mut em_paragrafo = false;
    for linha in texto.lines() {
        let marca = cerca_de_codigo(linha);
        if let Some(aberta) = cerca {
            // Fecha com o mesmo caractere, no mínimo do mesmo tamanho, e mais nada.
            let fecha = marca.is_some_and(|marca| {
                marca.starts_with(&aberta[..1])
                    && marca.len() >= aberta.len()
                    && linha.trim() == marca
            });
            if fecha {
                cerca = None;
            }
            continue;
        }
        if marca.is_some() {
            cerca = marca;
            em_paragrafo = false;
            continue;
        }
        match marcador_de_bloco(linha) {
            Some(Marcador::Abre) => {
                blocos_abertos += 1;
                em_paragrafo = false;
                continue;
            }
            Some(Marcador::Fecha) if blocos_abertos > 0 => {
                blocos_abertos -= 1;
                em_paragrafo = false;
                continue;
            }
            // Um `:::` sem bloco aberto é texto comum.
            Some(Marcador::Fecha) | None => {}
        }
        match titulo_da_linha(linha) {
            Some(titulo) if !em_paragrafo => {
                if !titulo.is_empty() {
                    return Some(titulo);
                }
            }
            Some(_) | None => em_paragrafo = !linha.trim().is_empty(),
        }
    }
    None
}

/// A sequência de três ou mais crases ou tis que começa a linha, com até
/// três espaços de recuo.
fn cerca_de_codigo(linha: &str) -> Option<&str> {
    let sem_recuo = linha.trim_start_matches(' ');
    if linha.len() - sem_recuo.len() > 3 {
        return None;
    }
    let caractere = sem_recuo
        .chars()
        .next()
        .filter(|c| matches!(c, '`' | '~'))?;
    let tamanho = sem_recuo.chars().take_while(|c| *c == caractere).count();
    (tamanho >= 3).then(|| &sem_recuo[..tamanho])
}

/// Uma linha `:::` do Pandoc: com algo depois dos dois-pontos abre um bloco,
/// sozinha fecha.
enum Marcador {
    Abre,
    Fecha,
}

fn marcador_de_bloco(linha: &str) -> Option<Marcador> {
    let dois_pontos = linha.chars().take_while(|c| *c == ':').count();
    if dois_pontos < 3 {
        return None;
    }
    if linha[dois_pontos..].trim().is_empty() {
        Some(Marcador::Fecha)
    } else {
        Some(Marcador::Abre)
    }
}

/// O texto de uma linha de título: um ou mais `#` na primeira coluna,
/// seguidos de espaço ou do fim da linha. Vazio para um título sem texto
/// (`#` sozinho), que para o Pandoc também é título.
fn titulo_da_linha(linha: &str) -> Option<String> {
    let cerquilhas = linha.chars().take_while(|c| *c == '#').count();
    let resto = &linha[cerquilhas..];
    if cerquilhas == 0 || !(resto.is_empty() || resto.starts_with(char::is_whitespace)) {
        return None;
    }
    // O Pandoc tira as cerquilhas do fim, coladas no texto ou não: `# Um #`
    // e `# Um#` são "Um".
    let sem_fecho = resto.trim().trim_end_matches('#');
    // Os asteriscos de ênfase não fazem parte do nome do capítulo.
    Some(sem_fecho.replace('*', "").trim().to_owned())
}
