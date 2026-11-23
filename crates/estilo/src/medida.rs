//! Medidas absolutas. Só entram unidades que o CSS e o Word entendem do mesmo
//! jeito; `em` e `%` ficam de fora porque o Word não tem equivalente direto.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::erro::ErroEstilo;

/// Maior medida que o Word aceita em recuo e espaçamento: 22 polegadas.
const TWIPS_MAXIMO: f64 = 31_680.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unidade {
    Pt,
    Cm,
    Mm,
    In,
}

impl Unidade {
    fn sufixo(self) -> &'static str {
        match self {
            Unidade::Pt => "pt",
            Unidade::Cm => "cm",
            Unidade::Mm => "mm",
            Unidade::In => "in",
        }
    }

    /// Twips (1/20 de ponto) por unidade. 1 in = 72 pt = 1440 twips.
    fn twips_por_unidade(self) -> f64 {
        match self {
            Unidade::Pt => 20.0,
            Unidade::Cm => 1440.0 / 2.54,
            Unidade::Mm => 1440.0 / 25.4,
            Unidade::In => 1440.0,
        }
    }
}

/// Medida não negativa, guardada como o autor escreveu: o CSS recebe o texto
/// original e o Word recebe a conversão para twips.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Medida {
    valor: f64,
    unidade: Unidade,
}

impl Medida {
    /// Valor em twips, arredondado: a unidade de recuo e espaçamento do Word.
    pub fn twips(self) -> u32 {
        // O intervalo 0..=31680 foi validado na leitura, então a conversão não trunca.
        (self.valor * self.unidade.twips_por_unidade()).round() as u32
    }

    /// Valor em pontos, para o tamanho de fonte.
    pub fn pontos(self) -> f64 {
        self.valor * self.unidade.twips_por_unidade() / 20.0
    }
}

impl fmt::Display for Medida {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.valor, self.unidade.sufixo())
    }
}

impl FromStr for Medida {
    type Err = ErroEstilo;

    fn from_str(texto: &str) -> Result<Self, Self::Err> {
        let invalida = |motivo: &str| ErroEstilo::MedidaInvalida {
            recebido: texto.to_owned(),
            motivo: motivo.to_owned(),
        };
        let aparado = texto.trim();
        let (numero, unidade) = [Unidade::Pt, Unidade::Cm, Unidade::Mm, Unidade::In]
            .into_iter()
            .find_map(|unidade| Some((aparado.strip_suffix(unidade.sufixo())?, unidade)))
            .ok_or_else(|| invalida("falta a unidade"))?;
        if numero.contains(',') {
            return Err(invalida("use ponto decimal, não vírgula"));
        }
        // Só dígitos e um ponto: sem sinal, expoente, espaço, "inf" ou "nan",
        // que o `f64::from_str` aceitaria.
        let so_digitos =
            |parte: &str| !parte.is_empty() && parte.bytes().all(|b| b.is_ascii_digit());
        let (inteira, decimal) = numero.split_once('.').unwrap_or((numero, "0"));
        if !so_digitos(inteira) || !so_digitos(decimal) {
            return Err(invalida("o número precisa ser como 12 ou 1.25"));
        }
        let valor: f64 = numero
            .parse()
            .map_err(|_| invalida("o número precisa ser como 12 ou 1.25"))?;
        if valor * unidade.twips_por_unidade() > TWIPS_MAXIMO {
            return Err(invalida("passa de 22 polegadas, o máximo do Word"));
        }
        Ok(Medida { valor, unidade })
    }
}

impl TryFrom<String> for Medida {
    type Error = ErroEstilo;

    fn try_from(texto: String) -> Result<Self, Self::Error> {
        texto.parse()
    }
}

impl From<Medida> for String {
    fn from(medida: Medida) -> Self {
        medida.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ler(texto: &str) -> Medida {
        match texto.parse::<Medida>() {
            Ok(medida) => medida,
            Err(erro) => panic!("{texto:?} deveria ser medida válida: {erro}"),
        }
    }

    #[test]
    fn converte_cada_unidade_para_twips() {
        assert_eq!(ler("12pt").twips(), 240);
        assert_eq!(ler("1.25cm").twips(), 709);
        assert_eq!(ler("10mm").twips(), 567);
        assert_eq!(ler("0.5in").twips(), 720);
        assert_eq!(ler("0pt").twips(), 0);
    }

    #[test]
    fn devolve_o_texto_que_o_autor_escreveu() {
        assert_eq!(ler("1.25cm").to_string(), "1.25cm");
        assert_eq!(ler(" 12pt ").to_string(), "12pt");
    }

    #[test]
    fn rejeita_o_que_o_word_nao_representa() {
        for texto in [
            "", "12", "pt", "1.5em", "50%", "-1cm", "1,25cm", "NaNpt", "infpt", "1e2pt", "+1cm",
            "23in", "1 cm",
        ] {
            assert!(
                texto.parse::<Medida>().is_err(),
                "{texto:?} deveria ser rejeitada"
            );
        }
    }

    #[test]
    fn erro_mostra_o_valor_recebido() {
        let erro = match "1,25cm".parse::<Medida>() {
            Ok(medida) => panic!("deveria falhar, veio {medida}"),
            Err(erro) => erro.to_string(),
        };
        assert!(erro.contains("1,25cm"), "{erro}");
        assert!(erro.contains("ponto"), "{erro}");
    }
}
