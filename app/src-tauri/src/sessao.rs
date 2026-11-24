//! O que o app tem aberto: um livro e os capítulos já lidos. Sem tauri aqui,
//! para a lógica ser testada sem abrir janela; `main.rs` só repassa.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use estilo::{Campos, CamposTrocados, Estilo, SeletorCss, gerar_css};
use serde::{Deserialize, Serialize};
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
    /// O aviso da pasta sem `estilo.yaml`, à parte dos outros: a interface
    /// põe nele o botão que cria o estilo. `None` quando o livro tem um
    /// estilo, mesmo inválido.
    pub aviso_sem_estilo: Option<String>,
}

/// Erro de um comando, no formato que a interface entende.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErroDeComando {
    /// `mudou-por-fora` pede uma decisão do autor; `erro` é só mostrar.
    pub codigo: &'static str,
    pub mensagem: String,
}

/// O estilo do livro como o painel "Estilo" o edita, na forma do
/// `estilo.yaml`: o corpo com todas as propriedades, e cada estilo nomeado só
/// com as que ele troca (o resto ele herda do corpo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstiloNoPainel {
    pub corpo: Campos,
    pub estilos: BTreeMap<String, CamposTrocados>,
    /// O livro já tem `estilo.yaml`. Sem ele, o painel mostra o modelo.
    pub existe: bool,
}

/// O capítulo que o autor acabou de criar, e o livro relido com ele na lista.
#[derive(Debug, PartialEq, Serialize)]
pub struct CapituloCriado {
    pub nome: String,
    pub livro: LivroAberto,
}

/// O livro renomeado, e o caminho que ele tinha: o app troca o que lembra
/// daquela pasta (a lista de livros, onde o autor parou) para a nova.
#[derive(Debug, PartialEq, Serialize)]
pub struct LivroRenomeado {
    pub antes: String,
    pub livro: LivroAberto,
}

/// O estilo que o botão "Criar estilo.yaml" grava: o exemplo comentado do
/// projeto, embutido no programa. É um arquivo só, para o exemplo que o
/// README manda copiar e o que o app cria nunca divergirem.
const MODELO_DE_ESTILO: &str = include_str!("../../../exemplos/estilo.yaml");

#[derive(Debug, Default)]
pub struct Sessao {
    livro: Option<Livro>,
    capitulos: HashMap<String, Capitulo>,
    /// O texto do `estilo.yaml` como o painel de estilo o leu ou gravou por
    /// último (`Some(None)`: não existia). `None`: o painel não foi aberto
    /// neste livro.
    estilo_lido: Option<Option<String>>,
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
        self.estilo_lido = None;
        Ok(aberto)
    }

    /// O livro aberto, relido da pasta, ou `None` se não há nenhum.
    pub fn livro_atual(&self) -> Result<Option<LivroAberto>, ErroDeComando> {
        self.livro.as_ref().map(montar).transpose()
    }

    /// Cria o `estilo.yaml` do livro aberto a partir do modelo e devolve o
    /// livro relido, já com o CSS. Se o arquivo já existe, é erro e nada muda.
    pub fn criar_estilo(&self) -> Result<LivroAberto, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de criar o estilo"));
        };
        livro.criar_estilo(MODELO_DE_ESTILO)?;
        montar(livro)
    }

    /// O estilo do livro para o painel "Estilo" mostrar: o do `estilo.yaml`
    /// ou, sem ele, o modelo. Só lê; nada é gravado ao abrir o painel. A
    /// sessão guarda o texto lido, para a gravação saber se o arquivo mudou
    /// por fora.
    pub fn estilo_para_editar(&mut self) -> Result<EstiloNoPainel, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de mexer no estilo"));
        };
        let lido = livro.ler_estilo()?;
        let estilo = Estilo::de_yaml(lido.as_deref().unwrap_or(MODELO_DE_ESTILO)).map_err(
            |motivo| {
                erro(&format!(
                    "O estilo.yaml deste livro tem um erro, e o painel não consegue mostrá-lo. \
                     Conserte o arquivo num editor de texto, ou apague-o para começar do modelo. {motivo}"
                ))
            },
        )?;
        let painel = EstiloNoPainel {
            corpo: Campos::from(&estilo.corpo),
            estilos: estilo
                .estilos
                .iter()
                .map(|(nome, paragrafo)| {
                    (
                        nome.clone(),
                        CamposTrocados::entre(paragrafo, &estilo.corpo),
                    )
                })
                .collect(),
            existe: lido.is_some(),
        };
        self.estilo_lido = Some(lido);
        Ok(painel)
    }

    /// Grava o estilo que o painel mostra e devolve o livro com o CSS novo.
    /// Valor que o Word não guarda é recusado, e o arquivo que mudou por fora
    /// desde que o painel abriu não é trocado.
    pub fn salvar_estilo(&mut self, painel: EstiloNoPainel) -> Result<LivroAberto, ErroDeComando> {
        let (Some(livro), Some(lido)) = (&self.livro, &self.estilo_lido) else {
            return Err(erro("abra o painel de estilo antes de mudar o estilo"));
        };
        let EstiloNoPainel {
            corpo,
            estilos,
            existe: _,
        } = painel;
        let estilo = Estilo::de_campos(corpo, estilos)
            .map_err(|motivo| erro(&erro_para_o_painel(&motivo.to_string())))?;
        let novo = estilo
            .para_yaml()
            .map_err(|motivo| erro(&motivo.to_string()))?;
        livro.gravar_estilo(&novo, lido.as_deref())?;
        let aberto = montar(livro)?;
        self.estilo_lido = Some(Some(novo));
        Ok(aberto)
    }

    /// Cria um capítulo vazio no livro aberto. O nome vem do autor e é
    /// conferido no `workspace`; a página não manda caminho.
    pub fn criar_capitulo(&self, nome: &str) -> Result<CapituloCriado, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de criar um capítulo"));
        };
        let nome = livro.criar_capitulo(nome)?;
        Ok(CapituloCriado {
            nome,
            livro: montar(livro)?,
        })
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

    /// Troca o nome da pasta do livro aberto (ADR 0003, "Vários livros").
    /// Os capítulos lidos eram do caminho velho: são esquecidos, e a página
    /// reabre o que estava na tela, depois de ter salvo o que faltava.
    pub fn renomear_livro(&mut self, nome: &str) -> Result<LivroRenomeado, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de renomeá-lo"));
        };
        let antes = livro.pasta().display().to_string();
        let renomeado = livro.renomear(nome)?;
        let aberto = montar(&renomeado)?;
        self.livro = Some(renomeado);
        self.capitulos.clear();
        self.estilo_lido = None;
        Ok(LivroRenomeado {
            antes,
            livro: aberto,
        })
    }

    /// Troca o nome do arquivo de um capítulo. Se ele estava aberto, o nome
    /// antigo deixa de valer: a página reabre pelo novo.
    pub fn renomear_capitulo(
        &mut self,
        atual: &str,
        novo: &str,
    ) -> Result<CapituloCriado, ErroDeComando> {
        let Some(livro) = &self.livro else {
            return Err(erro("abra a pasta do livro antes de renomear um capítulo"));
        };
        let nome = livro.renomear_capitulo(atual, novo)?;
        let aberto = montar(livro)?;
        if nome != atual {
            self.capitulos.remove(atual);
        }
        Ok(CapituloCriado {
            nome,
            livro: aberto,
        })
    }

    /// Cria a pasta de um livro novo, vazia, em `onde` (que veio do seletor
    /// do sistema), e abre nela.
    pub fn criar_livro(&mut self, onde: &Path, nome: &str) -> Result<LivroAberto, ErroDeComando> {
        let criado = workspace::criar_livro(onde, nome)?;
        self.abrir_pasta(criado.pasta())
    }

    /// A pasta do livro aberto, se há um.
    pub fn pasta(&self) -> Option<&Path> {
        self.livro.as_ref().map(Livro::pasta)
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
    let mut aviso_sem_estilo = None;
    let (css, fontes) = match estilo_do_livro(livro) {
        Ok(Some(estilo)) => (Some(css_do(&estilo)), fontes_do(&estilo)),
        Ok(None) => {
            aviso_sem_estilo = Some(
                "Este livro ainda não tem estilo.yaml, o arquivo que diz a fonte, o tamanho, \
                 a entrelinha e o recuo do texto. Sem ele a tela usa uma aparência padrão, e o \
                 .docx sai com o estilo padrão do Pandoc."
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
        aviso_sem_estilo,
    })
}

/// O nome do livro é o nome da pasta dele.
/// O erro de validação do estilo, dito para quem está no formulário: sem o
/// "estilo.yaml inválido" do começo nem a linha e a coluna do fim, que são do
/// texto que o formulário virou por dentro e o autor nunca viu.
fn erro_para_o_painel(mensagem: &str) -> String {
    let sem_prefixo = mensagem
        .strip_prefix("estilo.yaml inválido: ")
        .unwrap_or(mensagem);
    let sem_posicao = sem_prefixo
        .rfind(" (linha ")
        .map_or(sem_prefixo, |corte| &sem_prefixo[..corte]);
    format!("Esse valor não serve, e nada foi gravado. {sem_posicao}")
}

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
        assert_eq!(livro.aviso_sem_estilo, None);
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

    // Até 2026-11-24 o aviso da pasta sem estilo vinha em `avisos`, como os
    // outros, e mandava copiar "o exemplos/estilo.yaml do md Novel" — um
    // arquivo que o autor não sabia onde ficava. Agora ele vem à parte, em
    // `aviso_sem_estilo`, porque a interface põe nele o botão que cria o
    // estilo. `avisos` continua com os demais (estilo inválido, pasta sem
    // capítulo), que não têm botão.
    #[test]
    fn pasta_sem_estilo_abre_com_o_aviso_que_oferece_criar_e_sem_css() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let livro = Sessao::default().abrir_pasta(pasta.path()).expect("abrir");
        assert_eq!(livro.css, None);
        assert_eq!(nomes(&livro), ["01.md"]);
        assert_eq!(livro.avisos, Vec::<String>::new());
        let aviso = livro.aviso_sem_estilo.expect("aviso da pasta sem estilo");
        assert!(aviso.contains("estilo.yaml"), "{aviso}");
        // Diz para que serve o arquivo, sem mandar procurar outro.
        assert!(aviso.contains("fonte"), "{aviso}");
        assert!(!aviso.contains("exemplos/"), "{aviso}");
    }

    #[test]
    fn criar_estilo_grava_o_modelo_e_devolve_o_livro_com_o_css() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");

        let livro = sessao.criar_estilo().expect("criar o estilo");

        // O modelo é o `exemplos/estilo.yaml` do projeto, byte a byte.
        let modelo = include_bytes!("../../../exemplos/estilo.yaml");
        assert_eq!(
            fs::read(pasta.path().join("estilo.yaml")).expect("ler o estilo"),
            modelo
        );
        assert!(livro.css.is_some(), "o modelo é um estilo válido");
        assert_eq!(livro.aviso_sem_estilo, None);
        assert_eq!(livro.avisos, Vec::<String>::new());
        assert_eq!(
            livro.fontes.first().map(String::as_str),
            Some("EB Garamond")
        );
        assert_eq!(sessao.livro_atual().expect("atual"), Some(livro));
    }

    #[test]
    fn criar_estilo_quando_ja_existe_e_erro_e_nao_muda_o_arquivo() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        // Criado por fora depois de o livro abrir (outro computador, Dropbox).
        fs::write(pasta.path().join("estilo.yaml"), ESTILO).expect("criar por fora");

        let erro = sessao.criar_estilo().expect_err("já existe");

        assert_eq!(erro.codigo, "erro");
        assert!(erro.mensagem.contains("já existe"), "{erro:?}");
        assert_eq!(
            fs::read_to_string(pasta.path().join("estilo.yaml")).expect("ler"),
            ESTILO
        );
    }

    #[test]
    fn criar_capitulo_devolve_o_nome_do_arquivo_e_o_livro_com_ele_na_lista() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");

        let criado = sessao.criar_capitulo("02-a-casa").expect("criar");

        assert_eq!(criado.nome, "02-a-casa.md");
        assert_eq!(nomes(&criado.livro), ["01.md", "02-a-casa.md"]);
        assert_eq!(criado.livro.capitulos[1].palavras, Some(0));
        assert_eq!(
            fs::read(pasta.path().join("02-a-casa.md")).expect("ler"),
            b""
        );
        // O capítulo novo abre e salva como os outros.
        assert_eq!(sessao.abrir_capitulo("02-a-casa.md").expect("abrir"), "");
        sessao
            .salvar_capitulo("02-a-casa.md", "# A casa\n")
            .expect("salvar");

        let erro = sessao.criar_capitulo("../fora").expect_err("sai da pasta");
        assert_eq!(erro.codigo, "erro");
        assert!(erro.mensagem.contains("../fora"), "{erro:?}");
        let sem_livro = Sessao::default()
            .criar_capitulo("03")
            .expect_err("sem livro");
        assert!(sem_livro.mensagem.contains("pasta"), "{sem_livro:?}");
    }

    #[test]
    fn o_painel_abre_com_o_estilo_do_livro_e_grava_o_que_o_autor_muda() {
        let pasta = pasta_com(&[("01.md", "um"), ("estilo.yaml", ESTILO)]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");

        let mut painel = sessao.estilo_para_editar().expect("abrir o painel");
        assert!(painel.existe);
        assert_eq!(painel.corpo.fonte, "EB Garamond");
        assert_eq!(painel.corpo.tamanho, "12pt");
        assert_eq!(painel.estilos["Epígrafe"].italico, Some(true));
        assert_eq!(
            painel.estilos["Carta"].fonte.as_deref(),
            Some("Courier Prime")
        );
        // O estilo nomeado chega só com o que troca: o resto ele herda.
        assert_eq!(painel.estilos["Epígrafe"].fonte, None);
        assert_eq!(painel.estilos["Epígrafe"].tamanho, None);

        painel.corpo.tamanho = "14pt".to_owned();
        painel.corpo.alinhamento = "esquerda".to_owned();
        let livro = sessao.salvar_estilo(painel.clone()).expect("gravar");

        let css = livro.css.expect("css");
        assert!(css.contains("font-size: calc(14pt"), "{css}");
        assert!(css.contains("text-align: left"), "{css}");
        let gravado = fs::read_to_string(pasta.path().join("estilo.yaml")).expect("ler");
        assert!(gravado.contains("\n  tamanho: 14pt\n"), "{gravado}");
        // A epígrafe continua herdando: o tamanho novo vale para ela também.
        assert!(!gravado.contains("tamanho: 12pt"), "{gravado}");
        // Gravar de novo, em seguida, continua valendo: o painel acompanha o que ele mesmo gravou.
        painel.corpo.negrito = true;
        sessao.salvar_estilo(painel).expect("gravar de novo");
    }

    #[test]
    fn sem_estilo_o_painel_abre_com_o_modelo_e_o_primeiro_ajuste_cria_o_arquivo() {
        let pasta = pasta_com(&[("01.md", "um")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");

        let mut painel = sessao.estilo_para_editar().expect("abrir o painel");
        assert!(!painel.existe);
        assert_eq!(painel.corpo.fonte, "EB Garamond", "o modelo");
        assert!(
            !pasta.path().join("estilo.yaml").exists(),
            "abrir o painel não grava"
        );

        painel.corpo.fonte = "Liberation Serif".to_owned();
        let livro = sessao.salvar_estilo(painel).expect("gravar");
        assert_eq!(livro.aviso_sem_estilo, None);
        assert_eq!(
            livro.fontes.first().map(String::as_str),
            Some("Liberation Serif")
        );
        assert!(pasta.path().join("estilo.yaml").is_file());
    }

    #[test]
    fn o_painel_nao_grava_por_cima_do_que_mudou_por_fora_nem_valor_invalido() {
        let pasta = pasta_com(&[("01.md", "um"), ("estilo.yaml", ESTILO)]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        let mut painel = sessao.estilo_para_editar().expect("abrir o painel");

        // Valor que o Word não guarda: nada é gravado, e o erro diz o valor.
        let mut invalido = painel.clone();
        invalido.corpo.tamanho = "5000pt".to_owned();
        let erro = sessao.salvar_estilo(invalido).expect_err("inválido");
        assert!(erro.mensagem.contains("5000pt"), "{erro:?}");
        // Dito para quem está no formulário: sem a linha do arquivo.
        assert!(erro.mensagem.contains("nada foi gravado"), "{erro:?}");
        assert!(!erro.mensagem.contains("linha"), "{erro:?}");
        assert!(!erro.mensagem.contains("inválido:"), "{erro:?}");
        assert_eq!(
            fs::read_to_string(pasta.path().join("estilo.yaml")).expect("ler"),
            ESTILO
        );

        // Mudou por fora depois de o painel abrir.
        let por_fora = ESTILO.replace("12pt", "13pt");
        fs::write(pasta.path().join("estilo.yaml"), &por_fora).expect("mudar por fora");
        painel.corpo.negrito = true;
        let erro = sessao.salvar_estilo(painel).expect_err("mudou por fora");
        assert!(erro.mensagem.contains("mudou no disco"), "{erro:?}");
        assert_eq!(
            fs::read_to_string(pasta.path().join("estilo.yaml")).expect("ler"),
            por_fora
        );
    }

    #[test]
    fn o_painel_nao_abre_com_estilo_invalido_nem_grava_sem_ter_aberto() {
        let ruim = ESTILO.replace("1.25cm", "1,25cm");
        let pasta = pasta_com(&[("01.md", "um"), ("estilo.yaml", &ruim)]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        let erro = sessao.estilo_para_editar().expect_err("inválido");
        assert!(erro.mensagem.contains("1,25cm"), "{erro:?}");

        let outra = pasta_com(&[("01.md", "um"), ("estilo.yaml", ESTILO)]);
        sessao.abrir_pasta(outra.path()).expect("abrir");
        let painel = Sessao::default().estilo_para_editar();
        assert!(painel.is_err(), "sem livro");
        // Trocar de livro esquece o painel: gravar sem abrir de novo é erro,
        // mesmo quando os dois livros têm o estilo.yaml igualzinho (os dois
        // começaram do modelo) e a conferência do disco não notaria nada.
        let gemeo = pasta_com(&[("01.md", "um"), ("estilo.yaml", ESTILO)]);
        let mut de_outra = Sessao::default();
        de_outra.abrir_pasta(outra.path()).expect("abrir");
        let mut aberto = de_outra.estilo_para_editar().expect("abrir o painel");
        aberto.corpo.negrito = true;
        de_outra.abrir_pasta(gemeo.path()).expect("trocar de livro");
        let erro = de_outra
            .salvar_estilo(aberto)
            .expect_err("painel de outro livro");
        assert!(erro.mensagem.contains("painel"), "{erro:?}");
        assert_eq!(
            fs::read_to_string(gemeo.path().join("estilo.yaml")).expect("ler"),
            ESTILO
        );
    }

    #[test]
    fn criar_estilo_sem_livro_aberto_e_erro() {
        let erro = Sessao::default().criar_estilo().expect_err("sem livro");
        assert_eq!(erro.codigo, "erro");
        assert!(erro.mensagem.contains("pasta"), "{erro:?}");
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
        // O estilo existe, só está errado: nada de botão para criar outro.
        assert_eq!(livro.aviso_sem_estilo, None);
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

    // Renomear e criar livro (ADR 0003, "Vários livros").

    #[test]
    fn renomear_o_livro_aberto_muda_a_pasta_e_o_capitulo_reaberto_salva_no_lugar_novo() {
        let base = pasta_com(&[]);
        let pasta = base.path().join("livro");
        fs::create_dir(&pasta).expect("criar livro");
        fs::write(pasta.join("01.md"), "# Um\n").expect("criar capítulo");
        let mut sessao = Sessao::default();
        let aberto = sessao.abrir_pasta(&pasta).expect("abrir");
        sessao.abrir_capitulo("01.md").expect("abrir capítulo");

        let renomeado = sessao.renomear_livro("O livro").expect("renomear");
        let nova = base.path().canonicalize().expect("real").join("O livro");
        assert_eq!(renomeado.antes, aberto.pasta);
        assert_eq!(renomeado.livro.nome, "O livro");
        assert_eq!(renomeado.livro.pasta, nova.display().to_string());
        assert_eq!(nomes(&renomeado.livro), ["01.md"]);
        assert!(!pasta.exists(), "a pasta antiga não existe mais");
        assert_eq!(
            sessao
                .livro_atual()
                .expect("atual")
                .map(|livro| livro.pasta),
            Some(nova.display().to_string())
        );

        // O capítulo lido antes era do caminho velho: precisa ser reaberto,
        // e salvar sem reabrir não grava em lugar nenhum.
        assert!(sessao.salvar_capitulo("01.md", "# Um\n\nMais.\n").is_err());
        assert!(!pasta.exists(), "nada foi recriado no caminho velho");
        assert_eq!(sessao.abrir_capitulo("01.md").expect("reabrir"), "# Um\n");
        assert_eq!(
            sessao
                .salvar_capitulo("01.md", "# Um\n\nMais.\n")
                .expect("salvar"),
            Gravacao::Gravado
        );
        assert_eq!(
            fs::read_to_string(nova.join("01.md")).expect("ler"),
            "# Um\n\nMais.\n"
        );
    }

    #[test]
    fn renomear_o_capitulo_aberto_troca_o_nome_que_vale() {
        let pasta = pasta_com(&[("01.md", "# Um\n"), ("02.md", "# Dois\n")]);
        let mut sessao = Sessao::default();
        sessao.abrir_pasta(pasta.path()).expect("abrir");
        sessao.abrir_capitulo("01.md").expect("abrir capítulo");

        let renomeado = sessao
            .renomear_capitulo("01.md", "01 Início")
            .expect("renomear");
        assert_eq!(renomeado.nome, "01 Início.md");
        assert_eq!(nomes(&renomeado.livro), ["01 Início.md", "02.md"]);
        // O nome antigo não vale mais, nem para salvar.
        assert!(sessao.abrir_capitulo("01.md").is_err());
        assert!(sessao.salvar_capitulo("01.md", "outro").is_err());
        assert_eq!(
            sessao.abrir_capitulo("01 Início.md").expect("abrir o novo"),
            "# Um\n"
        );
        assert_eq!(
            sessao
                .salvar_capitulo("01 Início.md", "# Um\n\nMais.\n")
                .expect("salvar"),
            Gravacao::Gravado
        );
        assert!(!pasta.path().join("01.md").exists());
    }

    #[test]
    fn criar_livro_abre_a_pasta_nova_vazia_e_sem_estilo() {
        let base = pasta_com(&[]);
        let mut sessao = Sessao::default();
        let livro = sessao
            .criar_livro(base.path(), "Livro novo")
            .expect("criar");
        assert_eq!(livro.nome, "Livro novo");
        assert!(livro.capitulos.is_empty());
        assert!(livro.aviso_sem_estilo.is_some());
        // É o livro aberto: o capítulo novo nasce nele.
        sessao.criar_capitulo("01").expect("criar capítulo");
        assert!(base.path().join("Livro novo/01.md").is_file());
    }

    #[test]
    fn renomear_sem_livro_aberto_e_erro() {
        let mut sessao = Sessao::default();
        for resultado in [
            sessao.renomear_livro("x").map(|_| ()),
            sessao.renomear_capitulo("01.md", "x").map(|_| ()),
        ] {
            let erro = resultado.expect_err("sem livro");
            assert!(erro.mensagem.contains("abra"), "{}", erro.mensagem);
        }
    }
}
