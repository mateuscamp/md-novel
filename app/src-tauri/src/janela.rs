//! O que a janela faz além do livro: zoom e tela cheia. Aqui fica a regra,
//! sem tauri; os comandos de `main.rs` só repassam.

use crate::sessao::ErroDeComando;

/// A faixa do zoom, em por cento. A mesma de `app/src/conforto.ts`.
pub const ZOOM_MINIMO: u32 = 50;
pub const ZOOM_MAXIMO: u32 = 300;

/// O fator que o WebView recebe. A página manda só um número, e ele é
/// conferido aqui: um zoom absurdo deixaria a janela sem uso.
pub fn fator_de_zoom(por_cento: u32) -> Result<f64, ErroDeComando> {
    if !(ZOOM_MINIMO..=ZOOM_MAXIMO).contains(&por_cento) {
        return Err(ErroDeComando {
            codigo: "erro",
            mensagem: format!(
                "o zoom precisa ficar entre {ZOOM_MINIMO}% e {ZOOM_MAXIMO}%; recebi {por_cento}%"
            ),
        });
    }
    Ok(f64::from(por_cento) / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_fator_e_o_por_cento_dividido_por_cem() {
        for (por_cento, fator) in [(100, 1.0), (50, 0.5), (300, 3.0), (110, 1.1)] {
            let dado = fator_de_zoom(por_cento).expect("dentro da faixa");
            assert!((dado - fator).abs() < 1e-9, "{por_cento}: {dado}");
        }
    }

    #[test]
    fn zoom_fora_da_faixa_e_recusado_com_o_valor_e_a_faixa() {
        for ruim in [0, 49, 301, 100_000] {
            let erro = fator_de_zoom(ruim).expect_err("fora da faixa");
            assert_eq!(erro.codigo, "erro");
            assert!(erro.mensagem.contains(&ruim.to_string()), "{erro:?}");
            assert!(erro.mensagem.contains("50"), "{erro:?}");
            assert!(erro.mensagem.contains("300"), "{erro:?}");
        }
    }
}
