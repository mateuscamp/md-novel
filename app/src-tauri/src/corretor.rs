//! O corretor ortográfico. No Linux o WebKitGTK só sublinha palavra errada
//! se o app ligar a correção e disser a língua, e só há correção se o
//! dicionário dessa língua estiver instalado. Aqui fica a regra, sem tauri;
//! quem fala com o WebKitGTK é o `main.rs`.

/// A língua do corretor. É a mesma que o editor declara no texto (`pt-BR`).
pub const LINGUA: &str = "pt_BR";

/// O que dizer ao autor quando o dicionário da língua pedida não foi
/// carregado; `None` quando foi. `carregadas` é o que o WebKitGTK diz ter
/// carregado depois do pedido: ele deixa de fora a língua sem dicionário.
pub fn aviso_sem_dicionario(pedida: &str, carregadas: &[String]) -> Option<String> {
    if carregadas.iter().any(|lingua| lingua == pedida) {
        return None;
    }
    Some(format!(
        "O corretor ortográfico está desligado: não achei neste computador o dicionário \
         {pedida}. Instale o dicionário hunspell dessa língua (no Arch, o pacote \
         hunspell-pt-br, do AUR) e abra o md Novel de novo."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn com_o_dicionario_carregado_nao_ha_aviso() {
        assert_eq!(aviso_sem_dicionario(LINGUA, &["pt_BR".to_owned()]), None);
        assert_eq!(
            aviso_sem_dicionario(LINGUA, &["en_US".to_owned(), "pt_BR".to_owned()]),
            None
        );
    }

    #[test]
    fn sem_o_dicionario_da_lingua_pedida_o_aviso_diz_a_lingua_e_o_que_instalar() {
        // Nenhum carregado, ou só o de outra língua: o texto ficaria sem
        // correção, ou todo sublinhado.
        for carregadas in [vec![], vec!["en_US".to_owned()]] {
            let aviso = aviso_sem_dicionario(LINGUA, &carregadas).expect("aviso");
            assert!(aviso.contains("pt_BR"), "{aviso}");
            assert!(aviso.contains("hunspell"), "{aviso}");
            assert!(aviso.contains("corretor"), "{aviso}");
        }
    }
}
