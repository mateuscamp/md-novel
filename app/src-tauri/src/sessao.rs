//! O que o app tem aberto: um livro e os capítulos já lidos. Sem tauri aqui,
//! para a lógica ser testada sem abrir janela; `main.rs` só repassa.

use std::collections::HashMap;
use std::path::Path;

use estilo::{Estilo, SeletorCss, gerar_css};
use serde::Serialize;
use workspace::{Capitulo, ErroWorkspace, Gravacao, Livro};

use crate::exportacao::PlanoDeExportacao;

/// O seletor da linha de texto do CodeMirror: o CSS do livro vale direto
/// nela, sem tradução (ADR 0002 e 0003).
const SELETOR_DA_LINHA: &str = ".cm-content .cm-line";

/// Um capítulo na lateral do app.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CapituloNaLista {
    pub nome: String,
    pub titulo: Option<String>,
    pub palavras: Option<usize>,
}

/// O livro como a interface precisa: nomes, CSS pronto e avisos para o autor.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LivroAberto {
    pub pasta: String,
    pub nome: String,
    pub capitulos: Vec<CapituloNaLista>,
    /// CSS do `estilo.yaml`. `None` quando o livro não tem estilo válido.
    pub css: Option<String>,
    /// Fontes que o estilo pede, para a interface avisar se alguma falta.
    pub fontes: Vec<String>,
    pub avisos: Vec<String>,
}

/// Erro de um comando, no formato que a interface entende.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErroDeComando {
    /// `mudou-por-fora` pede uma decisão do autor; `erro` é só mostrar.
    pub codigo: &'static str,
    pub mensagem: String,
}

#[derive(Debug, Default)]
pub struct Sessao {
    livro: Option<Livro>,
    capitulos: HashMap<String, Capitulo>,
}

impl Sessao {
    /// Troca o livro aberto. Os capítulos lidos do livro anterior são esquecidos.
    pub fn abrir_pasta(&mut self, pasta: &Path) -> Result<LivroAberto, ErroDeComando> {
        // O caminho real: `md-novel-app .` mostra o nome da pasta, não ".".
        // Se não der para resolver, `Livro::abrir` explica o motivo.
        let real = pasta.canonicalize().unwrap_or_else(|_| pasta.to_owned());
        let livro = Livro::abrir(&real)?;
        let aberto = montar(&livro)?;
        self.livro = Some(livro);
        self.capitulos.clear();
        Ok(aberto)
    }

    /// O livro aberto, relido da pasta, ou `None` se não há nenhum.
    pub fn livro_atual(&self) -> Result<Option<LivroAberto>, ErroDeComando> {
        self.livro.as_ref().map(montar).transpose()
    }

    /// Lê o capítulo do disco e passa a acompanhá-lo. Abrir de novo relê:
    /// é assim que o autor resolve um "mudou por fora".
    pub fn abrir_capitulo(&mut self, nome: &str) -> Result<String, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de abrir um capítulo"));
        };
        let capitulo = livro.ler_capitulo(nome)?;
        let texto = capitulo.texto().to_owned();
        self.capitulos.insert(nome.to_owned(), capitulo);
        Ok(texto)
    }

    /// O que exportar agora: os capítulos em ordem e o estilo, relidos da pasta.
    pub fn plano_de_exportacao(&self) -> Result<PlanoDeExportacao, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de exportar"));
        };
        let pasta = livro.pasta().to_owned();
        Ok(PlanoDeExportacao {
            nome: nome_da(&pasta),
            capitulos: livro
                .capitulos()?
                .iter()
                .map(|capitulo| pasta.join(capitulo))
                .collect(),
            estilo: livro.ler_estilo()?,
            pasta,
        })
    }

    pub fn salvar_capitulo(&mut self, nome: &str, texto: &str) -> Result<Gravacao, ErroDeComando> {
        // Só se salva o que foi lido nesta sessão: é a leitura que guarda os
        // bytes para saber se o arquivo mudou por fora.
        let Some(capitulo) = self.capitulos.get_mut(nome) else {
            return Err(erro(&format!(
                "o capítulo {nome:?} não está aberto; abra-o antes de salvar"
            )));
        };
        Ok(capitulo.salvar(texto)?)
    }
}

/// Junta o que a interface precisa. Estilo ausente ou inválido vira aviso:
/// o autor continua podendo abrir e editar os capítulos.
fn montar(livro: &Livro) -> Result<LivroAberto, ErroDeComando> {
    let capitulos: Vec<CapituloNaLista> = livro
        .resumos()?
        .into_iter()
        .map(|resumo| {
            // Sem `..`: campo novo no resumo quebra a compilação aqui.
            let workspace::ResumoDoCapitulo {
                nome,
                titulo,
                palavras,
            } = resumo;
            CapituloNaLista {
                nome,
                titulo,
                palavras,
            }
        })
        .collect();
    let mut avisos = Vec::new();
    if capitulos.is_empty() {
        avisos.push(
            "Esta pasta não tem nenhum capítulo: os capítulos são os arquivos .md dela.".to_owned(),
        );
    }
    let (css, fontes) = match estilo_do_livro(livro) {
        Ok(Some(estilo)) => (Some(css_do(&estilo)), fontes_do(&estilo)),
        Ok(None) => {
            avisos.push(
                "Esta pasta não tem estilo.yaml: o editor está com a aparência padrão. \
                 Copie o exemplos/estilo.yaml do md Novel para cá e ajuste."
                    .to_owned(),
            );
            (None, Vec::new())
        }
        Err(mensagem) => {
            avisos.push(format!(
                "O estilo.yaml não pôde ser usado, e o editor está com a aparência padrão. {mensagem}"
            ));
            (None, Vec::new())
        }
    };
    let pasta = livro.pasta();
    Ok(LivroAberto {
        pasta: pasta.display().to_string(),
        nome: nome_da(pasta),
        capitulos,
        css,
        fontes,
        avisos,
    })
}

/// O nome do livro é o nome da pasta dele.
fn nome_da(pasta: &Path) -> String {
    pasta.file_name().map_or_else(
        || pasta.display().to_string(),
        |nome| nome.to_string_lossy().into_owned(),
    )
}

fn estilo_do_livro(livro: &Livro) -> Result<Option<Estilo>, String> {
    let Some(texto) = livro.ler_estilo().map_err(|erro| erro.to_string())? else {
        return Ok(None);
    };
    Estilo::de_yaml(&texto)
        .map(Some)
        .map_err(|erro| erro.to_string())
}

fn css_do(estilo: &Estilo) -> String {
    // A constante é um seletor válido; se deixar de ser, o padrão do crate
    // mantém o CSS bem formado e o teste do seletor reprova.
    let seletor = SeletorCss::novo(SELETOR_DA_LINHA).unwrap_or_default();
    gerar_css(estilo, &seletor)
}

/// As fontes que o estilo pede, a do corpo primeiro, sem repetir.
fn fontes_do(estilo: &Estilo) -> Vec<String> {
    let mut fontes: Vec<String> = Vec::new();
    let paragrafos = std::iter::once(&estilo.corpo).chain(estilo.estilos.values());
    for paragrafo in paragrafos {
        let nome = paragrafo.fonte.nome();
        if !fontes.iter().any(|fonte| fonte == nome) {
            fontes.push(nome.to_owned());
        }
    }
    fontes
}

fn erro(mensagem: &str) -> ErroDeComando {
    ErroDeComando {
        codigo: "erro",
        mensagem: mensagem.to_owned(),
    }
}

impl From<ErroWorkspace> for ErroDeComando {
    fn from(origem: ErroWorkspace) -> Self {
        let codigo = if matches!(origem, ErroWorkspace::MudouPorFora { .. }) {
            "mudou-por-fora"
        } else {
            "erro"
        };
        ErroDeComando {
            codigo,
            mensagem: origem.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const ESTILO: &str = "versao: 1
corpo:
  fonte: EB Garamond
  tamanho: 12pt
  entrelinha: 1.5
  alinhamento: justificado
  recuo-primeira-linha: 1.25cm
  espaco-antes: 0pt
  espaco-depois: 0pt
estilos:
  Carta:
    fonte: Courier Prime
  Epígrafe:
    italico: true
";

    fn nomes(livro: &LivroAberto) -> Vec<&str> {
        livro
            .capitulos
            .iter()
            .map(|capitulo| capitulo.nome.as_str())
            .collect()
    }

    #[test]
    fn a_lista_traz_o_titulo_e_as_palavras_de_cada_capitulo() {
        let pasta = pasta_com(&[
            ("01.md", "# A casa\n\nEla abria as janelas.\n"),
            ("02.md", "sem título"),
        ]);
        let livro = Sessao::default().abrir_pasta(pasta.path()).expect("abrir");
        assert_eq!(
            livro.capitulos,
            [
                CapituloNaLista {
                    nome: "01.md".to_owned(),
                    titulo: Some("A casa".to_owned()),
                    palavras: Some(6),
                },
                CapituloNaLista {
                    nome: "02.md".to_owned(),
                    titulo: None,
                    palavras: Some(2),
                },
            ]
        );
    }

    fn pasta_com(arquivos: &[(&str, &str)]) -> tempfile::TempDir {
        let pasta = tempfile::tempdir().expect("pasta temporária");
        for (nome, conteudo) in arquivos {
            fs::write(pasta.path().join(nome), conteudo).expect("criar arquivo");
        }
        pasta
    }

    #[test]
    fn abrir_pasta_entrega_capitulos_css_da_linha_do_editor_e_fontes() {
        let pasta = pasta_com(&[("02.md", "dois"), ("01.md", "um"), ("estilo.yaml", ESTILO)]);
        let mut sessao = Sessao::default();

        let livro = sessao.abrir_pasta(pasta.path()).expect("abrir");

        // Até 2026-11-23 `capitulos` era só a lista de nomes. A lateral do app
        // passou a mostrar título e palavras, então cada item virou um resumo.
        assert_eq!(nomes(&livro), ["01.md", "02.md"]);
        assert_eq!(livro.capitulos[0].titulo, None);
        assert_eq!(livro.capitulos[0].palavras, Some(1));
        // O caminho real: no macOS a pasta temporária fica atrás de um link
        // (`/var` aponta para `/private/var`), e o app mostra o destino.
        let real = pasta.path().canonicalize().expect("caminho real");
        assert_eq!(livro.pasta, real.display().to_string());
        assert_eq!(
            livro.nome,
            pasta.path().file_name().expect("nome").to_string_lossy()
        );
        let css = livro.css.clone().expect("css do estilo");
        assert!(css.contains("\n.cm-content .cm-line {\n"), "{css}");
        assert!(
            css.contains(":is(.cm-content .cm-line)[data-md-novel-estilo=\"Epígrafe\"]"),
            "{css}"
        );
        assert_eq!(
            livro.fontes,
            ["EB Garamond", "Courier Prime"],
            "corpo primeiro, sem repetir"
        );
        assert_eq!(livro.avisos, Vec::<String>::new());
        assert_eq!(sessao.livro_atual().expect("atual"), Some(livro));
    }

    #[test]
    fn pasta_dada_por_caminho_relativo_aparece_com_o_nome_e_o_caminho_reais() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let real = pasta.path().canonicalize().expect("caminho real");
        let torto = pasta
            .path()
            .join(".")
            .join("..")
            .join(real.file_name().expect("nome"))
            .join(".");
        let livro = Sessao::default().abrir_pasta(&torto).expect("abrir");
        assert_eq!(
            livro.nome,
            real.file_name().expect("nome").to_string_lossy()
        );
        assert_eq!(livro.pasta, real.display().to_string());
    }

    #[test]
    fn sem_livro_aberto_nao_ha_livro_atual_nem_capitulo() {
        let mut sessao = Sessao::default();
        assert_eq!(sessao.livro_atual().expect("atual"), None);
        let erro = sessao.abrir_capitulo("01.md").expect_err("sem livro");
        assert!(erro.mensagem.contains("pasta"), "{erro:?}");
    }

    #[test]
    fn pasta_sem_estilo_abre_com_aviso_e_sem_css() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let livro = Sessao::default().abrir_pasta(pasta.path()).expect("abrir");
        assert_eq!(livro.css, None);
        assert_eq!(nomes(&livro), ["01.md"]);
        assert_eq!(livro.avisos.len(), 1, "{:?}", livro.avisos);
        assert!(
            livro.avisos[0].contains("estilo.yaml"),
            "{:?}",
            livro.avisos
        );
    }

    #[test]
    fn estilo_invalido_nao_impede_de_abrir_os_capitulos() {
        let ruim = ESTILO.replace("1.25cm", "1,25cm");
        let pasta = pasta_com(&[("01.md", "um"), ("estilo.yaml", &ruim)]);
        let livro = Sessao::default().abrir_pasta(pasta.path()).expect("abrir");
        assert_eq!(livro.css, None);
        assert_eq!(nomes(&livro), ["01.md"]);
        assert!(
            livro.avisos.iter().any(|aviso| aviso.contains("1,25cm")),
            "{:?}",
            livro.avisos
        );
    }

    #[test]
    fn pasta_sem_capitulo_avisa() {
        let pasta = pasta_com(&[("estilo.yaml", ESTILO)]);
        let livro = Sessao::default().abrir_pasta(pasta.path()).expect("abrir");
        assert!(
            livro.avisos.iter().any(|aviso| aviso.contains(".md")),
            "{:?}",
            livro.avisos
        );
    }

    #[test]
    fn caminho_que_nao_e_pasta_e_erro_e_mantem_o_livro_anterior() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        let erro = sessao
            .abrir_pasta(&pasta.path().join("01.md"))
            .expect_err("não é pasta");
        assert_eq!(erro.codigo, "erro");
        assert!(sessao.livro_atual().expect("atual").is_some());
    }

    #[test]
    fn abrir_editar_e_salvar_um_capitulo() {
        let pasta = pasta_com(&[("01.md", "Um.\r\n")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");

        assert_eq!(
            sessao.abrir_capitulo("01.md").expect("abrir capítulo"),
            "Um.\n"
        );
        assert_eq!(
            sessao.salvar_capitulo("01.md", "Um.\n").expect("salvar"),
            Gravacao::SemMudanca
        );
        assert_eq!(
            sessao
                .salvar_capitulo("01.md", "Um.\nDois.\n")
                .expect("salvar"),
            Gravacao::Gravado
        );

        assert_eq!(
            fs::read(pasta.path().join("01.md")).expect("reler"),
            b"Um.\r\nDois.\r\n"
        );
    }

    #[test]
    fn salvar_capitulo_que_nao_foi_aberto_e_erro() {
        let pasta = pasta_com(&[("01.md", "Um.\n")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        let erro = sessao
            .salvar_capitulo("01.md", "outro texto")
            .expect_err("não aberto");
        assert_eq!(erro.codigo, "erro");
        assert_eq!(
            fs::read_to_string(pasta.path().join("01.md")).expect("reler"),
            "Um.\n"
        );
    }

    #[test]
    fn mudanca_por_fora_vira_codigo_proprio_e_reabrir_resolve() {
        let pasta = pasta_com(&[("01.md", "Um.\n")]);
        let caminho = pasta.path().join("01.md");
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        sessao.abrir_capitulo("01.md").expect("abrir capítulo");
        fs::write(&caminho, "Um, do outro computador.\n").expect("mudança externa");

        let erro = sessao
            .salvar_capitulo("01.md", "Um, daqui.\n")
            .expect_err("mudou por fora");
        assert_eq!(erro.codigo, "mudou-por-fora");
        assert_eq!(
            fs::read_to_string(&caminho).expect("reler"),
            "Um, do outro computador.\n"
        );

        assert_eq!(
            sessao.abrir_capitulo("01.md").expect("reabrir"),
            "Um, do outro computador.\n"
        );
        assert_eq!(
            sessao
                .salvar_capitulo("01.md", "Um, daqui.\n")
                .expect("salvar"),
            Gravacao::Gravado
        );
    }

    #[test]
    fn trocar_de_livro_esquece_os_capitulos_do_anterior() {
        let primeiro = pasta_com(&[("01.md", "Um.\n")]);
        let segundo = pasta_com(&[("01.md", "Outro livro.\n")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(primeiro.path()).expect("abrir");
        sessao.abrir_capitulo("01.md").expect("abrir capítulo");
        sessao.abrir_pasta(segundo.path()).expect("abrir o segundo");

        let erro = sessao
            .salvar_capitulo("01.md", "texto do primeiro livro")
            .expect_err("não aberto no segundo");
        assert_eq!(erro.codigo, "erro");
        assert_eq!(
            fs::read_to_string(segundo.path().join("01.md")).expect("reler"),
            "Outro livro.\n"
        );
    }
}
