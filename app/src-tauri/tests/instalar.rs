//! `app/linux/instalar.sh`, o que o `make instalar` e o `make desinstalar`
//! chamam: põe o programa, o ícone e a entrada do menu num prefixo e tira de
//! lá. Os testes rodam o script de verdade num prefixo temporário, com um
//! arquivo qualquer no lugar do programa compilado.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const PROGRAMA: &str = "bin/md-novel-app";

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// O identificador do app, lido de onde ele é decidido: o nome dos arquivos
/// instalados tem de ser o mesmo.
fn identificador() -> String {
    let configuracao =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json"))
            .expect("ler o tauri.conf.json");
    let inicio = configuracao
        .find("\"identifier\": \"")
        .expect("achar o identificador no tauri.conf.json")
        + "\"identifier\": \"".len();
    let fim = configuracao[inicio..]
        .find('"')
        .expect("fim do identificador");
    configuracao[inicio..inicio + fim].to_owned()
}

fn entrada_do_menu() -> String {
    format!("share/applications/{}.desktop", identificador())
}

fn icone() -> String {
    format!("share/icons/hicolor/64x64/apps/{}.png", identificador())
}

/// Uma pasta com um `uname` que responde `sistema`, para pôr na frente do
/// PATH: o script pergunta ao `uname` onde está, e assim os mesmos testes
/// valem no Linux e no Mac do CI.
fn uname_de_mentira(pasta: &Path, sistema: &str) -> PathBuf {
    let bin = pasta.join("uname-de-mentira");
    fs::create_dir_all(&bin).expect("criar a pasta do uname de mentira");
    let uname = bin.join("uname");
    fs::write(&uname, format!("#!/bin/sh\necho {sistema}\n")).expect("escrever o uname de mentira");
    fs::set_permissions(&uname, fs::Permissions::from_mode(0o755)).expect("tornar executável");
    bin
}

/// Onde o script cria o rascunho da entrada do menu: dentro do apoio, para
/// o teste ver se ele limpou.
fn rascunhos(apoio: &Path) -> PathBuf {
    apoio.join("rascunhos")
}

fn script() -> PathBuf {
    raiz().join("app/linux/instalar.sh")
}

fn rodar_em(sistema: &str, apoio: &Path, argumentos: &[&str]) -> Output {
    rodar_o_script(&script(), sistema, apoio, argumentos)
}

fn rodar_o_script(script: &Path, sistema: &str, apoio: &Path, argumentos: &[&str]) -> Output {
    let caminho = std::env::var("PATH").expect("PATH");
    fs::create_dir_all(rascunhos(apoio)).expect("criar a pasta dos rascunhos");
    Command::new("sh")
        .arg(script)
        .args(argumentos)
        .env(
            "PATH",
            format!(
                "{}:{caminho}",
                uname_de_mentira(apoio, sistema).to_str().expect("utf-8")
            ),
        )
        .env("TMPDIR", rascunhos(apoio))
        .output()
        .expect("rodar o instalar.sh")
}

fn rodar(apoio: &Path, argumentos: &[&str]) -> Output {
    rodar_em("Linux", apoio, argumentos)
}

fn erro(saida: &Output) -> String {
    String::from_utf8_lossy(&saida.stderr).into_owned()
}

/// Um prefixo com espaço no nome (o caminho vai entre aspas na entrada do
/// menu) e um arquivo fazendo as vezes do programa compilado.
struct Bancada {
    apoio: TempDir,
    prefixo: PathBuf,
    compilado: PathBuf,
}

impl Bancada {
    fn nova() -> Self {
        let apoio = tempfile::tempdir().expect("pasta temporária");
        let prefixo = apoio.path().join("pasta do autor/.local");
        let compilado = apoio.path().join("md-novel-app");
        fs::write(&compilado, b"programa, primeira compilacao").expect("escrever o programa");
        Self {
            apoio,
            prefixo,
            compilado,
        }
    }

    fn instalar(&self) -> Output {
        rodar(
            self.apoio.path(),
            &[
                "instalar",
                self.prefixo.to_str().expect("utf-8"),
                self.compilado.to_str().expect("utf-8"),
            ],
        )
    }

    fn desinstalar(&self) -> Output {
        rodar(
            self.apoio.path(),
            &["desinstalar", self.prefixo.to_str().expect("utf-8")],
        )
    }

    /// Todo arquivo que existe no prefixo, em ordem.
    fn arquivos(&self) -> Vec<String> {
        fn descer(pasta: &Path, base: &Path, achados: &mut Vec<String>) {
            let Ok(entradas) = fs::read_dir(pasta) else {
                return;
            };
            for entrada in entradas {
                let caminho = entrada.expect("ler a entrada").path();
                if caminho.is_dir() {
                    descer(&caminho, base, achados);
                } else {
                    let relativo = caminho.strip_prefix(base).expect("dentro do prefixo");
                    achados.push(relativo.to_str().expect("utf-8").to_owned());
                }
            }
        }
        let mut achados = Vec::new();
        descer(&self.prefixo, &self.prefixo, &mut achados);
        achados.sort();
        achados
    }

    /// O que sobrou na pasta dos rascunhos.
    fn rascunhos_esquecidos(&self) -> Vec<PathBuf> {
        fs::read_dir(rascunhos(self.apoio.path()))
            .expect("ler a pasta dos rascunhos")
            .map(|entrada| entrada.expect("ler a entrada").path())
            .collect()
    }
}

#[test]
fn instala_o_programa_o_icone_e_a_entrada_do_menu_e_mais_nada() {
    let bancada = Bancada::nova();
    let saida = bancada.instalar();
    assert!(saida.status.success(), "{}", erro(&saida));

    let mut esperados = vec![PROGRAMA.to_owned(), entrada_do_menu(), icone()];
    esperados.sort();
    assert_eq!(bancada.arquivos(), esperados);
    assert_eq!(bancada.rascunhos_esquecidos(), Vec::<PathBuf>::new());

    let programa = bancada.prefixo.join(PROGRAMA);
    assert_eq!(
        fs::read(&programa).expect("ler o programa instalado"),
        b"programa, primeira compilacao"
    );
    let modo = fs::metadata(&programa)
        .expect("metadados")
        .permissions()
        .mode();
    assert_eq!(modo & 0o777, 0o755, "o programa instalado é executável");

    assert_eq!(
        fs::read(bancada.prefixo.join(icone())).expect("ler o ícone instalado"),
        fs::read(raiz().join("app/src-tauri/icons/icon.png")).expect("ler o ícone do projeto"),
    );

    let prefixo = bancada.prefixo.to_str().expect("utf-8");
    assert_eq!(
        fs::read_to_string(bancada.prefixo.join(entrada_do_menu())).expect("ler a entrada do menu"),
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=md Novel\n\
             Comment=Escrever um livro em Markdown, com o estilo do .docx na tela\n\
             Exec=\"{prefixo}/{PROGRAMA}\"\n\
             Icon={prefixo}/{}\n\
             Terminal=false\n\
             Categories=Office;WordProcessor;\n\
             StartupWMClass=md-novel-app\n",
            icone()
        )
    );
}

#[test]
fn instalar_de_novo_troca_o_programa_pelo_recem_compilado() {
    let bancada = Bancada::nova();
    assert!(bancada.instalar().status.success());
    fs::write(&bancada.compilado, b"programa, segunda compilacao").expect("recompilar");
    let saida = bancada.instalar();
    assert!(saida.status.success(), "{}", erro(&saida));

    assert_eq!(
        fs::read(bancada.prefixo.join(PROGRAMA)).expect("ler o programa instalado"),
        b"programa, segunda compilacao"
    );
    assert_eq!(bancada.arquivos().len(), 3, "{:?}", bancada.arquivos());
}

#[test]
fn sem_o_programa_compilado_recusa_diz_como_compilar_e_nao_escreve_nada() {
    let bancada = Bancada::nova();
    fs::remove_file(&bancada.compilado).expect("apagar o programa");
    let saida = bancada.instalar();

    assert!(!saida.status.success());
    let mensagem = erro(&saida);
    assert!(
        mensagem.contains(bancada.compilado.to_str().expect("utf-8")),
        "{mensagem}"
    );
    assert!(
        mensagem.contains("cargo tauri build --no-bundle"),
        "{mensagem}"
    );
    assert!(bancada.arquivos().is_empty(), "{:?}", bancada.arquivos());
}

#[test]
fn sem_o_icone_do_projeto_recusa_antes_de_escrever() {
    let bancada = Bancada::nova();
    // Uma cópia do script fora do projeto: o ícone, que ele procura a partir
    // de onde está, não existe ali.
    let copia = bancada.apoio.path().join("copia/app/linux/instalar.sh");
    fs::create_dir_all(copia.parent().expect("pasta da cópia")).expect("criar a pasta da cópia");
    fs::copy(script(), &copia).expect("copiar o script");

    let saida = rodar_o_script(
        &copia,
        "Linux",
        bancada.apoio.path(),
        &[
            "instalar",
            bancada.prefixo.to_str().expect("utf-8"),
            bancada.compilado.to_str().expect("utf-8"),
        ],
    );
    assert!(!saida.status.success());
    let mensagem = erro(&saida);
    assert!(
        mensagem.contains("não achei o ícone do projeto"),
        "{mensagem}"
    );
    assert!(mensagem.contains("src-tauri/icons/icon.png"), "{mensagem}");
    assert!(!bancada.prefixo.exists(), "{:?}", bancada.arquivos());
    assert_eq!(bancada.rascunhos_esquecidos(), Vec::<PathBuf>::new());
}

#[test]
fn falha_no_meio_nao_publica_nenhum_dos_tres_nem_deixa_temporario() {
    let bancada = Bancada::nova();
    // Um arquivo onde deveria haver a pasta dos ícones: o programa ainda dá
    // para preparar, o ícone não.
    fs::create_dir_all(bancada.prefixo.join("share")).expect("criar share");
    fs::write(bancada.prefixo.join("share/icons"), b"no caminho").expect("criar o obstáculo");

    let saida = bancada.instalar();
    assert!(!saida.status.success());
    let mensagem = erro(&saida);
    assert!(mensagem.contains(&icone()), "{mensagem}");
    assert_eq!(bancada.arquivos(), ["share/icons"]);
    assert_eq!(bancada.rascunhos_esquecidos(), Vec::<PathBuf>::new());
}

#[test]
fn prefixo_que_nao_cabe_na_entrada_do_menu_e_recusado_com_o_valor_recebido() {
    let bancada = Bancada::nova();
    let base = bancada.apoio.path().to_str().expect("utf-8").to_owned();
    // Relativo, e os caracteres que a especificação da entrada do menu
    // reserva dentro de aspas ou no nome do programa.
    let recusados = [
        "relativo/.local".to_owned(),
        format!("{base}/com\"aspas"),
        format!("{base}/com`crase"),
        format!("{base}/com$cifrao"),
        format!("{base}/com\\barra"),
        format!("{base}/com%porcento"),
        format!("{base}/com=igual"),
        format!("{base}/com\nquebra"),
        format!("{base}/com\ttabulacao"),
    ];
    for prefixo in recusados {
        let saida = rodar(
            bancada.apoio.path(),
            &[
                "instalar",
                &prefixo,
                bancada.compilado.to_str().expect("utf-8"),
            ],
        );
        assert!(!saida.status.success(), "aceitou o prefixo {prefixo:?}");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("prefixo"), "{prefixo:?}: {mensagem}");
        assert!(
            mensagem.contains(prefixo.trim_end()),
            "{prefixo:?}: {mensagem}"
        );
        assert!(
            !Path::new(&prefixo).exists(),
            "criou a pasta do prefixo recusado {prefixo:?}"
        );
    }
}

#[test]
fn fora_do_linux_recusa_dizendo_o_sistema_e_nao_escreve_nada() {
    let bancada = Bancada::nova();
    let prefixo = bancada.prefixo.to_str().expect("utf-8");
    let compilado = bancada.compilado.to_str().expect("utf-8");
    let chamadas: [&[&str]; 3] = [
        &["instalar", prefixo, compilado],
        &["desinstalar", prefixo],
        &["conferir", prefixo],
    ];
    for argumentos in chamadas {
        let comando = argumentos[0];
        let saida = rodar_em("Darwin", bancada.apoio.path(), argumentos);
        assert!(!saida.status.success(), "{comando} passou fora do Linux");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("Linux"), "{comando}: {mensagem}");
        assert!(mensagem.contains("Darwin"), "{comando}: {mensagem}");
        assert!(bancada.arquivos().is_empty(), "{:?}", bancada.arquivos());
    }
}

#[test]
fn conferir_aceita_o_que_instalar_aceitaria_sem_escrever_nada() {
    let bancada = Bancada::nova();
    let saida = rodar(
        bancada.apoio.path(),
        &["conferir", bancada.prefixo.to_str().expect("utf-8")],
    );
    assert!(saida.status.success(), "{}", erro(&saida));
    assert!(!bancada.prefixo.exists());
}

#[test]
fn desinstalar_apaga_so_os_tres_arquivos_que_instalar_criou() {
    let bancada = Bancada::nova();
    assert!(bancada.instalar().status.success());
    // Vizinhos que não são do md Novel, nas mesmas pastas.
    let vizinhos = [
        "bin/outro-programa",
        "share/applications/outro.desktop",
        "share/icons/hicolor/64x64/apps/outro.png",
    ];
    for vizinho in vizinhos {
        fs::write(bancada.prefixo.join(vizinho), b"de outro programa").expect("criar o vizinho");
    }

    let saida = bancada.desinstalar();
    assert!(saida.status.success(), "{}", erro(&saida));
    assert_eq!(bancada.arquivos(), vizinhos);

    // Sem nada instalado, desinstalar de novo não é erro nem apaga mais nada.
    let de_novo = bancada.desinstalar();
    assert!(de_novo.status.success(), "{}", erro(&de_novo));
    assert_eq!(bancada.arquivos(), vizinhos);
}

#[test]
fn comando_desconhecido_ou_incompleto_mostra_o_uso_com_o_que_recebeu() {
    let bancada = Bancada::nova();
    let prefixo = bancada.prefixo.to_str().expect("utf-8");
    let chamadas: [&[&str]; 4] = [
        &[],
        &["apagar", prefixo],
        &["instalar", prefixo],
        &["desinstalar"],
    ];
    for argumentos in chamadas {
        let saida = rodar(bancada.apoio.path(), argumentos);
        assert!(!saida.status.success(), "aceitou {argumentos:?}");
        let mensagem = erro(&saida);
        assert!(mensagem.contains("uso:"), "{argumentos:?}: {mensagem}");
        let recebido = argumentos.first().copied().unwrap_or("recebi: nada");
        assert!(mensagem.contains(recebido), "{argumentos:?}: {mensagem}");
        assert!(bancada.arquivos().is_empty(), "{:?}", bancada.arquivos());
    }
}
