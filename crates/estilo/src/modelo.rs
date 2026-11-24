//! O schema do `estilo.yaml` e a resolução da herança: cada estilo nomeado
//! parte do corpo e troca só o que declarar.

use std::collections::BTreeMap;

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};

use crate::erro::ErroEstilo;
use crate::medida::Medida;

const VERSAO_SUPORTADA: u32 = 1;
const NOME_DE_ESTILO_MAXIMO: usize = 64;
const FONTE_MAXIMO_DE_CARACTERES: usize = 64;
/// Maior múltiplo de entrelinha que o Word aceita.
const ENTRELINHA_MAXIMA: f64 = 132.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alinhamento {
    Esquerda,
    Centro,
    Direita,
    Justificado,
}

/// Nome de fonte que cabe sem escape numa string CSS e num atributo XML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Fonte(String);

impl Fonte {
    pub fn nome(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Fonte {
    type Error = ErroEstilo;

    fn try_from(texto: String) -> Result<Self, Self::Error> {
        let nome = texto.trim();
        // `<` e `>` ficam de fora porque o editor vai injetar este CSS numa
        // página: um nome de fonte não pode fechar a tag de estilo.
        let proibido = |c: char| c.is_control() || matches!(c, '"' | '\\' | '\'' | '<' | '>');
        if nome.is_empty()
            || nome.chars().count() > FONTE_MAXIMO_DE_CARACTERES
            || nome.chars().any(proibido)
        {
            return Err(ErroEstilo::ValorInvalido {
                chave: "fonte",
                recebido: texto,
                esperado: "o nome de uma fonte, com até 64 caracteres e sem aspas, barra invertida, < ou >",
            });
        }
        Ok(Fonte(nome.to_owned()))
    }
}

impl From<Fonte> for String {
    fn from(fonte: Fonte) -> Self {
        fonte.0
    }
}

/// Tamanho de fonte que o Word consegue guardar em meios-pontos.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Medida", into = "Medida")]
pub struct TamanhoFonte(Medida);

impl TamanhoFonte {
    pub fn medida(self) -> Medida {
        self.0
    }

    /// Meios-pontos, a unidade de `w:sz`.
    pub fn meios_pontos(self) -> u32 {
        // Múltiplo de 0.5pt entre 0.5pt e 1584pt, validado na leitura.
        (self.0.pontos() * 2.0).round() as u32
    }
}

impl TryFrom<Medida> for TamanhoFonte {
    type Error = ErroEstilo;

    fn try_from(medida: Medida) -> Result<Self, Self::Error> {
        // O Word guarda o tamanho em meios-pontos inteiros. Um tamanho fora
        // dessa grade seria arredondado só no .docx e a tela mostraria outro,
        // então é recusado. Não há teto aqui: a medida já para em 22 polegadas
        // (1584pt), abaixo do máximo de 1638pt do Word.
        let meios_pontos = medida.pontos() * 2.0;
        let na_grade = (meios_pontos - meios_pontos.round()).abs() < 1e-6;
        if meios_pontos.round() < 1.0 || !na_grade {
            return Err(ErroEstilo::ValorInvalido {
                chave: "tamanho",
                recebido: medida.to_string(),
                esperado: "um tamanho de fonte múltiplo de 0.5pt, a partir de 0.5pt, como 12pt ou 10.5pt",
            });
        }
        Ok(TamanhoFonte(medida))
    }
}

impl From<TamanhoFonte> for Medida {
    fn from(tamanho: TamanhoFonte) -> Self {
        tamanho.0
    }
}

/// Entrelinha como múltiplo da linha simples da fonte, no sentido do Word:
/// 1.5 é o "1,5 linhas" da caixa de parágrafo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(into = "f64")]
pub struct Entrelinha(f64);

/// Leitura à mão em vez de `try_from = "f64"`: o YAML entrega `1,5` como
/// texto, e assim a resposta é a mensagem do projeto, com a linha do valor,
/// e não o erro de tipo do serde.
impl<'de> Deserialize<'de> for Entrelinha {
    fn deserialize<D: Deserializer<'de>>(leitor: D) -> Result<Self, D::Error> {
        struct Visitante;

        impl Visitor<'_> for Visitante {
            type Value = Entrelinha;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("um número, como 1.5")
            }

            fn visit_f64<E: de::Error>(self, multiplo: f64) -> Result<Entrelinha, E> {
                Entrelinha::try_from(multiplo).map_err(E::custom)
            }

            fn visit_i64<E: de::Error>(self, multiplo: i64) -> Result<Entrelinha, E> {
                self.visit_f64(multiplo as f64)
            }

            fn visit_u64<E: de::Error>(self, multiplo: u64) -> Result<Entrelinha, E> {
                self.visit_f64(multiplo as f64)
            }

            fn visit_str<E: de::Error>(self, texto: &str) -> Result<Entrelinha, E> {
                Err(E::custom(ErroEstilo::ValorInvalido {
                    chave: "entrelinha",
                    recebido: texto.to_owned(),
                    esperado: "um número com ponto decimal, sem aspas e sem vírgula, como 1.5",
                }))
            }
        }

        leitor.deserialize_any(Visitante)
    }
}

impl Entrelinha {
    pub fn multiplo(self) -> f64 {
        self.0
    }

    /// `w:line` com `w:lineRule="auto"`: 240 é a linha simples.
    pub fn duzentos_e_quarenta_avos(self) -> u32 {
        // O intervalo 0.25..=132 foi validado na leitura.
        (self.0 * 240.0).round() as u32
    }
}

impl TryFrom<f64> for Entrelinha {
    type Error = ErroEstilo;

    fn try_from(multiplo: f64) -> Result<Self, Self::Error> {
        if !(0.25..=ENTRELINHA_MAXIMA).contains(&multiplo) {
            return Err(ErroEstilo::ValorInvalido {
                chave: "entrelinha",
                recebido: multiplo.to_string(),
                esperado: "um múltiplo da linha simples entre 0.25 e 132, como 1.5",
            });
        }
        Ok(Entrelinha(multiplo))
    }
}

impl From<Entrelinha> for f64 {
    fn from(entrelinha: Entrelinha) -> Self {
        entrelinha.0
    }
}

/// Um estilo de parágrafo com todas as propriedades resolvidas.
///
/// É o subconjunto comum ao CSS e ao Word. Campo novo aqui quebra a
/// compilação de `css.rs` e `docx.rs` até os dois o traduzirem.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EstiloParagrafo {
    pub fonte: Fonte,
    pub tamanho: TamanhoFonte,
    pub entrelinha: Entrelinha,
    pub alinhamento: Alinhamento,
    pub recuo_primeira_linha: Medida,
    pub espaco_antes: Medida,
    pub espaco_depois: Medida,
    pub italico: bool,
    pub negrito: bool,
}

/// O que o autor escreveu num bloco: só as propriedades que quis declarar.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Parcial {
    fonte: Option<Fonte>,
    tamanho: Option<TamanhoFonte>,
    entrelinha: Option<Entrelinha>,
    alinhamento: Option<Alinhamento>,
    recuo_primeira_linha: Option<Medida>,
    espaco_antes: Option<Medida>,
    espaco_depois: Option<Medida>,
    italico: Option<bool>,
    negrito: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Documento {
    // Já conferida por `SoVersao`; fica aqui para o `deny_unknown_fields` aceitá-la.
    #[serde(rename = "versao")]
    _versao: u32,
    corpo: Parcial,
    #[serde(default)]
    estilos: BTreeMap<String, Parcial>,
}

/// O estilo de um livro: o corpo e os estilos nomeados, já resolvidos.
#[derive(Debug, Clone, PartialEq)]
pub struct Estilo {
    pub corpo: EstiloParagrafo,
    /// Nome do estilo como o autor usa em `::: {custom-style="Nome"}`.
    pub estilos: BTreeMap<String, EstiloParagrafo>,
}

impl Estilo {
    /// Lê e valida um `estilo.yaml`.
    ///
    /// ```
    /// let estilo = estilo::Estilo::de_yaml(
    ///     "versao: 1\ncorpo:\n  fonte: EB Garamond\n  tamanho: 12pt\n  entrelinha: 1.5\n  \
    ///      alinhamento: justificado\n  recuo-primeira-linha: 1.25cm\n  espaco-antes: 0pt\n  \
    ///      espaco-depois: 0pt\n",
    /// )
    /// .expect("estilo válido");
    /// assert_eq!(estilo.corpo.recuo_primeira_linha.twips(), 709);
    /// ```
    pub fn de_yaml(texto: &str) -> Result<Estilo, ErroEstilo> {
        // A versão é conferida antes do schema: um arquivo de versão futura
        // deve dizer "versão não suportada", não "campo desconhecido".
        let SoVersao { versao } = serde_yaml_ng::from_str(texto)?;
        if versao != VERSAO_SUPORTADA {
            return Err(ErroEstilo::VersaoNaoSuportada { recebida: versao });
        }
        let Documento {
            _versao: _,
            corpo,
            estilos,
        } = serde_yaml_ng::from_str(texto)?;
        let corpo = resolver(corpo, None)?;
        let mut resolvidos: BTreeMap<String, EstiloParagrafo> = BTreeMap::new();
        for (nome, parcial) in estilos {
            validar_nome_de_estilo(&nome)?;
            let chave = chave_de_colisao(&nome);
            if let Some(existente) = resolvidos
                .keys()
                .find(|outro| chave_de_colisao(outro) == chave)
            {
                return Err(ErroEstilo::NomeReservado {
                    nome,
                    existente: existente.clone(),
                });
            }
            let resolvido = resolver(parcial, Some(&corpo))?;
            resolvidos.insert(nome, resolvido);
        }
        Ok(Estilo {
            corpo,
            estilos: resolvidos,
        })
    }
}

/// As propriedades de um parágrafo como um formulário as edita (o painel
/// "Estilo" do app): todas presentes, com as medidas escritas como no
/// `estilo.yaml` (`12pt`, `1.25cm`). Não é validado aqui: quem valida é
/// [`Estilo::de_campos`], pelo mesmo caminho do arquivo.
///
/// Tem as mesmas chaves de [`EstiloParagrafo`]; `tests/painel.rs` confere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Campos {
    pub fonte: String,
    pub tamanho: String,
    pub entrelinha: f64,
    pub alinhamento: String,
    pub recuo_primeira_linha: String,
    pub espaco_antes: String,
    pub espaco_depois: String,
    pub italico: bool,
    pub negrito: bool,
}

impl From<&EstiloParagrafo> for Campos {
    fn from(paragrafo: &EstiloParagrafo) -> Self {
        // Sem `..`: propriedade nova quebra a compilação aqui até entrar no formulário.
        let EstiloParagrafo {
            fonte,
            tamanho,
            entrelinha,
            alinhamento,
            recuo_primeira_linha,
            espaco_antes,
            espaco_depois,
            italico,
            negrito,
        } = paragrafo;
        let alinhamento = match alinhamento {
            Alinhamento::Esquerda => "esquerda",
            Alinhamento::Centro => "centro",
            Alinhamento::Direita => "direita",
            Alinhamento::Justificado => "justificado",
        };
        Campos {
            fonte: fonte.nome().to_owned(),
            tamanho: tamanho.medida().to_string(),
            entrelinha: entrelinha.multiplo(),
            alinhamento: alinhamento.to_owned(),
            recuo_primeira_linha: recuo_primeira_linha.to_string(),
            espaco_antes: espaco_antes.to_string(),
            espaco_depois: espaco_depois.to_string(),
            italico: *italico,
            negrito: *negrito,
        }
    }
}

/// As propriedades que um estilo nomeado troca em relação ao corpo, como um
/// formulário as edita. O que falta aqui é herdado: mudar o corpo muda o
/// estilo nomeado junto, como num estilo "baseado em" do Word.
///
/// Tem as mesmas chaves de [`EstiloParagrafo`]; `tests/painel.rs` confere.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct CamposTrocados {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fonte: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tamanho: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrelinha: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alinhamento: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recuo_primeira_linha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espaco_antes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub espaco_depois: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italico: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negrito: Option<bool>,
}

impl CamposTrocados {
    /// O que `paragrafo` tem de diferente de `corpo`.
    pub fn entre(paragrafo: &EstiloParagrafo, corpo: &EstiloParagrafo) -> Self {
        // Sem `..`: propriedade nova quebra a compilação aqui até entrar no formulário.
        let Campos {
            fonte,
            tamanho,
            entrelinha,
            alinhamento,
            recuo_primeira_linha,
            espaco_antes,
            espaco_depois,
            italico,
            negrito,
        } = Campos::from(paragrafo);
        let base = Campos::from(corpo);
        CamposTrocados {
            fonte: (fonte != base.fonte).then_some(fonte),
            tamanho: (tamanho != base.tamanho).then_some(tamanho),
            entrelinha: (entrelinha != base.entrelinha).then_some(entrelinha),
            alinhamento: (alinhamento != base.alinhamento).then_some(alinhamento),
            recuo_primeira_linha: (recuo_primeira_linha != base.recuo_primeira_linha)
                .then_some(recuo_primeira_linha),
            espaco_antes: (espaco_antes != base.espaco_antes).then_some(espaco_antes),
            espaco_depois: (espaco_depois != base.espaco_depois).then_some(espaco_depois),
            italico: (italico != base.italico).then_some(italico),
            negrito: (negrito != base.negrito).then_some(negrito),
        }
    }
}

/// O que um estilo nomeado tem de diferente do corpo: é só isso que vai para
/// o arquivo, para ele continuar herdando o resto.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct Diferencas<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    fonte: Option<&'a Fonte>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tamanho: Option<TamanhoFonte>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entrelinha: Option<Entrelinha>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alinhamento: Option<Alinhamento>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recuo_primeira_linha: Option<Medida>,
    #[serde(skip_serializing_if = "Option::is_none")]
    espaco_antes: Option<Medida>,
    #[serde(skip_serializing_if = "Option::is_none")]
    espaco_depois: Option<Medida>,
    #[serde(skip_serializing_if = "Option::is_none")]
    italico: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negrito: Option<bool>,
}

fn diferencas<'a>(paragrafo: &'a EstiloParagrafo, corpo: &EstiloParagrafo) -> Diferencas<'a> {
    // Sem `..`: propriedade nova quebra a compilação aqui até ser escrita.
    let EstiloParagrafo {
        fonte,
        tamanho,
        entrelinha,
        alinhamento,
        recuo_primeira_linha,
        espaco_antes,
        espaco_depois,
        italico,
        negrito,
    } = paragrafo;
    Diferencas {
        fonte: (*fonte != corpo.fonte).then_some(fonte),
        tamanho: (*tamanho != corpo.tamanho).then_some(*tamanho),
        entrelinha: (*entrelinha != corpo.entrelinha).then_some(*entrelinha),
        alinhamento: (*alinhamento != corpo.alinhamento).then_some(*alinhamento),
        recuo_primeira_linha: (*recuo_primeira_linha != corpo.recuo_primeira_linha)
            .then_some(*recuo_primeira_linha),
        espaco_antes: (*espaco_antes != corpo.espaco_antes).then_some(*espaco_antes),
        espaco_depois: (*espaco_depois != corpo.espaco_depois).then_some(*espaco_depois),
        italico: (*italico != corpo.italico).then_some(*italico),
        negrito: (*negrito != corpo.negrito).then_some(*negrito),
    }
}

/// O começo do `estilo.yaml` que o programa grava. Os comentários de quem
/// editou à mão não sobrevivem a uma gravação, e o arquivo diz isso.
const CABECALHO: &str = "# Estilo do livro no md Novel: a fonte, o tamanho, a entrelinha, o\n\
# alinhamento, o recuo e os espaços do texto. O painel \"Estilo\" do programa\n\
# grava este arquivo. Editar à mão também vale, mas comentários escritos aqui\n\
# não são guardados quando o painel grava de novo.\n";

impl Estilo {
    /// O texto do `estilo.yaml` deste estilo: o corpo inteiro, e de cada
    /// estilo nomeado só o que difere do corpo. Ler o texto de volta dá o
    /// mesmo estilo.
    pub fn para_yaml(&self) -> Result<String, ErroEstilo> {
        #[derive(Serialize)]
        struct Escrito<'a> {
            versao: u32,
            corpo: &'a EstiloParagrafo,
            #[serde(skip_serializing_if = "BTreeMap::is_empty")]
            estilos: BTreeMap<&'a str, Diferencas<'a>>,
        }
        let escrito = Escrito {
            versao: VERSAO_SUPORTADA,
            corpo: &self.corpo,
            estilos: self
                .estilos
                .iter()
                .map(|(nome, paragrafo)| (nome.as_str(), diferencas(paragrafo, &self.corpo)))
                .collect(),
        };
        Ok(format!(
            "{CABECALHO}{}",
            serde_yaml_ng::to_string(&escrito)?
        ))
    }

    /// O estilo que um formulário descreve. Os campos passam pela mesma
    /// validação do arquivo: viram o texto de um `estilo.yaml` e são lidos
    /// por [`Estilo::de_yaml`].
    pub fn de_campos(
        corpo: Campos,
        estilos: BTreeMap<String, CamposTrocados>,
    ) -> Result<Estilo, ErroEstilo> {
        #[derive(Serialize)]
        struct Formulario {
            versao: u32,
            corpo: Campos,
            estilos: BTreeMap<String, CamposTrocados>,
        }
        let texto = serde_yaml_ng::to_string(&Formulario {
            versao: VERSAO_SUPORTADA,
            corpo,
            estilos,
        })?;
        Estilo::de_yaml(&texto)
    }
}

#[derive(Deserialize)]
struct SoVersao {
    versao: u32,
}

/// Junta o que o autor declarou com a base. Sem base (o corpo), toda
/// propriedade é obrigatória, menos itálico e negrito, que começam desligados.
fn resolver(
    parcial: Parcial,
    base: Option<&EstiloParagrafo>,
) -> Result<EstiloParagrafo, ErroEstilo> {
    // Sem `..`: campo novo em `Parcial` quebra a compilação aqui.
    let Parcial {
        fonte,
        tamanho,
        entrelinha,
        alinhamento,
        recuo_primeira_linha,
        espaco_antes,
        espaco_depois,
        italico,
        negrito,
    } = parcial;
    Ok(EstiloParagrafo {
        fonte: exigir(fonte, base.map(|b| b.fonte.clone()), "fonte")?,
        tamanho: exigir(tamanho, base.map(|b| b.tamanho), "tamanho")?,
        entrelinha: exigir(entrelinha, base.map(|b| b.entrelinha), "entrelinha")?,
        alinhamento: exigir(alinhamento, base.map(|b| b.alinhamento), "alinhamento")?,
        recuo_primeira_linha: exigir(
            recuo_primeira_linha,
            base.map(|b| b.recuo_primeira_linha),
            "recuo-primeira-linha",
        )?,
        espaco_antes: exigir(espaco_antes, base.map(|b| b.espaco_antes), "espaco-antes")?,
        espaco_depois: exigir(
            espaco_depois,
            base.map(|b| b.espaco_depois),
            "espaco-depois",
        )?,
        italico: italico.or(base.map(|b| b.italico)).unwrap_or(false),
        negrito: negrito.or(base.map(|b| b.negrito)).unwrap_or(false),
    })
}

fn exigir<T>(
    declarado: Option<T>,
    herdado: Option<T>,
    chave: &'static str,
) -> Result<T, ErroEstilo> {
    declarado
        .or(herdado)
        .ok_or(ErroEstilo::CorpoIncompleto { chave })
}

/// Dois nomes colidem quando dariam o mesmo estilo no Word: o id é o nome sem
/// espaços, e o Word não diferencia maiúsculas de minúsculas em nome de estilo.
pub(crate) fn chave_de_colisao(nome: &str) -> String {
    nome.replace(' ', "").to_lowercase()
}

/// O nome vai para uma string CSS e para um atributo XML; só entra o que
/// cabe nos dois sem escape.
fn validar_nome_de_estilo(nome: &str) -> Result<(), ErroEstilo> {
    let permitido = |c: char| c.is_alphanumeric() || c == ' ' || c == '-';
    let valido = !nome.is_empty()
        && nome == nome.trim()
        && nome.chars().count() <= NOME_DE_ESTILO_MAXIMO
        && nome.chars().all(permitido);
    if valido {
        Ok(())
    } else {
        Err(ErroEstilo::NomeDeEstiloInvalido {
            recebido: nome.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPO: &str = "versao: 1
corpo:
  fonte: EB Garamond
  tamanho: 12pt
  entrelinha: 1.5
  alinhamento: justificado
  recuo-primeira-linha: 1.25cm
  espaco-antes: 0pt
  espaco-depois: 0pt
";

    fn ler(yaml: &str) -> Estilo {
        match Estilo::de_yaml(yaml) {
            Ok(estilo) => estilo,
            Err(erro) => panic!("deveria ser válido: {erro}\n{yaml}"),
        }
    }

    fn erro_de(yaml: &str) -> String {
        match Estilo::de_yaml(yaml) {
            Ok(_) => panic!("deveria ser rejeitado:\n{yaml}"),
            Err(erro) => erro.to_string(),
        }
    }

    #[test]
    fn le_o_corpo_com_italico_e_negrito_desligados_por_padrao() {
        let corpo = ler(CORPO).corpo;
        assert_eq!(corpo.fonte.nome(), "EB Garamond");
        assert_eq!(corpo.tamanho.meios_pontos(), 24);
        assert_eq!(corpo.entrelinha.duzentos_e_quarenta_avos(), 360);
        assert_eq!(corpo.alinhamento, Alinhamento::Justificado);
        assert_eq!(corpo.recuo_primeira_linha.twips(), 709);
        assert!(!corpo.italico);
        assert!(!corpo.negrito);
    }

    #[test]
    fn estilo_nomeado_herda_do_corpo_e_troca_so_o_que_declara() {
        let yaml = format!(
            "{CORPO}estilos:\n  Epígrafe:\n    alinhamento: direita\n    italico: true\n    recuo-primeira-linha: 0cm\n"
        );
        let estilo = ler(&yaml);
        let epigrafe = &estilo.estilos["Epígrafe"];
        assert_eq!(epigrafe.alinhamento, Alinhamento::Direita);
        assert!(epigrafe.italico);
        assert_eq!(epigrafe.recuo_primeira_linha.twips(), 0);
        assert_eq!(epigrafe.fonte, estilo.corpo.fonte);
        assert_eq!(epigrafe.entrelinha, estilo.corpo.entrelinha);
    }

    #[test]
    fn rejeita_propriedade_fora_do_subconjunto_em_vez_de_ignorar() {
        let erro = erro_de(&format!("{CORPO}  viuvas: 2\n"));
        assert!(erro.contains("viuvas"), "{erro}");
        let erro = erro_de(&format!("{CORPO}pagina:\n  tamanho: A5\n"));
        assert!(erro.contains("pagina"), "{erro}");
    }

    #[test]
    fn corpo_sem_uma_propriedade_obrigatoria_e_rejeitado() {
        let sem_entrelinha = CORPO.replace("  entrelinha: 1.5\n", "");
        let erro = erro_de(&sem_entrelinha);
        assert!(erro.contains("entrelinha"), "{erro}");
    }

    #[test]
    fn rejeita_versao_desconhecida() {
        let erro = erro_de(&CORPO.replace("versao: 1", "versao: 2"));
        assert!(erro.contains("versão 2"), "{erro}");
    }

    #[test]
    fn rejeita_valores_fora_da_faixa_do_word() {
        for (de, para) in [
            ("tamanho: 12pt", "tamanho: 0pt"),
            ("entrelinha: 1.5", "entrelinha: 0"),
            ("entrelinha: 1.5", "entrelinha: .nan"),
            ("entrelinha: 1.5", "entrelinha: 200"),
            ("alinhamento: justificado", "alinhamento: justify"),
            ("fonte: EB Garamond", "fonte: 'Fonte \"x\"'"),
            ("fonte: EB Garamond", "fonte: ''"),
            (
                "recuo-primeira-linha: 1.25cm",
                "recuo-primeira-linha: 1.5em",
            ),
        ] {
            assert!(CORPO.contains(de), "fixture sem {de:?}");
            let yaml = CORPO.replace(de, para);
            assert!(
                Estilo::de_yaml(&yaml).is_err(),
                "{para:?} deveria ser rejeitado"
            );
        }
    }

    // O caso `2000pt` saiu da lista acima em 2026-11-23 (auditoria): ele era
    // recusado pelo teto de 22 polegadas da medida, não pela faixa de tamanho,
    // e passava pelo motivo errado. Aqui cada limite é conferido pela mensagem.
    #[test]
    fn tamanho_de_fonte_so_aceita_o_que_o_word_guarda() {
        for aceito in ["12pt", "10.5pt", "0.5pt", "1584pt"] {
            let yaml = CORPO.replace("tamanho: 12pt", &format!("tamanho: {aceito}"));
            ler(&yaml);
        }
        // O Word guarda meios-pontos: 4mm (11,34pt) viraria 11,5pt só no .docx.
        let erro = erro_de(&CORPO.replace("tamanho: 12pt", "tamanho: 4mm"));
        assert!(erro.contains("4mm") && erro.contains("0.5pt"), "{erro}");
        let erro = erro_de(&CORPO.replace("tamanho: 12pt", "tamanho: 10.2pt"));
        assert!(erro.contains("10.2pt") && erro.contains("0.5pt"), "{erro}");
        let erro = erro_de(&CORPO.replace("tamanho: 12pt", "tamanho: 0pt"));
        assert!(erro.contains("tamanho"), "{erro}");
        let erro = erro_de(&CORPO.replace("tamanho: 12pt", "tamanho: 2000pt"));
        assert!(erro.contains("22 polegadas"), "{erro}");
    }

    #[test]
    fn fonte_tem_limite_de_tamanho_e_nao_aceita_marcacao() {
        let no_limite = "F".repeat(64);
        ler(&CORPO.replace("EB Garamond", &no_limite));
        let acima = "F".repeat(65);
        erro_de(&CORPO.replace("EB Garamond", &acima));
        for fonte in ["'</style>'", "'a<b'", "'a>b'"] {
            erro_de(&CORPO.replace("EB Garamond", fonte));
        }
    }

    #[test]
    fn erros_de_digitacao_comuns_saem_em_portugues_com_o_valor_recebido() {
        let casos = [
            (
                CORPO.replace("entrelinha: 1.5", "entrelinha: 1,5"),
                vec!["entrelinha", "1,5", "ponto"],
            ),
            (
                CORPO.replace("entrelinha: 1.5", "entrelinha: [1]"),
                vec!["entrelinha", "número"],
            ),
            (
                format!("{CORPO}  viuvas: 2\n"),
                vec!["campo desconhecido", "viuvas", "esperado um de"],
            ),
            (
                CORPO.replace("alinhamento: justificado", "alinhamento: justify"),
                vec!["valor desconhecido", "justify", "justificado"],
            ),
            (
                CORPO.replace("corpo:", "korpo:"),
                vec!["campo desconhecido", "korpo"],
            ),
            ("versao: 1\n".to_owned(), vec!["falta o campo", "corpo"]),
            (
                format!("{CORPO}  italico: sim\n"),
                vec!["italico", "sim", "true ou false"],
            ),
            (
                CORPO.replace("tamanho: 12pt", "tamanho: 12"),
                vec!["12", "falta a unidade"],
            ),
        ];
        for (yaml, trechos) in casos {
            let erro = erro_de(&yaml);
            for trecho in trechos {
                assert!(erro.contains(trecho), "faltou {trecho:?} em: {erro}");
            }
            for ingles in [
                "invalid type",
                "unknown ",
                "expected",
                "missing field",
                " at line ",
                "untagged",
            ] {
                assert!(!erro.contains(ingles), "sobrou {ingles:?} em: {erro}");
            }
        }
    }

    #[test]
    fn erro_de_sintaxe_do_yaml_sai_em_portugues_com_a_linha_do_problema() {
        let com_tab = "versao: 1\ncorpo:\n\tfonte: x\n";
        let erro = erro_de(com_tab);
        assert!(erro.contains("tab"), "{erro}");
        assert!(erro.contains("(linha 3, coluna 1)"), "{erro}");
        let recuo_errado = "versao: 1\ncorpo:\n  fonte: EB Garamond\n tamanho: 12pt\n";
        let erro_de_recuo = erro_de(recuo_errado);
        assert!(erro_de_recuo.contains("indentação"), "{erro_de_recuo}");
        assert!(
            erro_de_recuo.contains("(linha 4, coluna 2)"),
            "{erro_de_recuo}"
        );
        for mensagem in [erro, erro_de_recuo] {
            for ingles in [
                " at line ",
                "while ",
                "found character",
                "did not find",
                "expected",
            ] {
                assert!(
                    !mensagem.contains(ingles),
                    "sobrou {ingles:?} em: {mensagem}"
                );
            }
        }
    }

    #[test]
    fn erro_diz_a_linha_do_estilo_yaml() {
        let erro = erro_de(&format!("{CORPO}  viuvas: 2\n"));
        assert!(erro.contains("(linha 10, coluna 3)"), "{erro}");
        // A linha é a do valor errado, não a do começo do bloco.
        let erro = erro_de(&CORPO.replace("entrelinha: 1.5", "entrelinha: 1,5"));
        assert!(erro.contains("(linha 5, coluna 15)"), "{erro}");
    }

    #[test]
    fn rejeita_nome_de_estilo_que_nao_cabe_no_css_e_no_word() {
        for nome in ["'a\"b'", "'a]b'", "''", "' '", "'<x>'"] {
            let yaml = format!("{CORPO}estilos:\n  {nome}:\n    italico: true\n");
            assert!(
                Estilo::de_yaml(&yaml).is_err(),
                "{nome} deveria ser rejeitado"
            );
        }
        let longo = "a".repeat(NOME_DE_ESTILO_MAXIMO + 1);
        let yaml = format!("{CORPO}estilos:\n  {longo}:\n    italico: true\n");
        assert!(Estilo::de_yaml(&yaml).is_err());
    }

    #[test]
    fn aceita_nome_de_estilo_com_acento_espaco_e_hifen() {
        let yaml = format!("{CORPO}estilos:\n  Carta do avô - 2:\n    italico: true\n");
        assert!(ler(&yaml).estilos.contains_key("Carta do avô - 2"));
    }
}
