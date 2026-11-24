"""Teste funcional do app de desktop: abre o programa de verdade numa pasta
de livro temporária, deixa a interface digitar uma frase e confere o que
chegou ao disco e o que o navegador aplicou.

Uso: python3 -I teste_funcional.py [--na-tela] [--passada=N] <binário md-novel-app> [png da janela]

`--passada=N` roda só a passada N (de 1 a 10), para conferir uma mudança sem
esperar as outras; dizer que a interface funciona pede as dez.

Por padrão o app abre num display virtual invisível (`xvfb-run`), para não
pôr janela na tela de quem está trabalhando. `--na-tela` abre na sessão
gráfica de verdade. O png, se pedido, é tirado depois de exportar; as
passadas 4 e 5 gravam ao lado dele `<nome>-escuro.png` e `<nome>-foco.png`.

Dez passadas:
1. salvamento automático: digita e espera o app salvar sozinho; confere
   também a lateral, a contagem de palavras ao vivo, o tema claro e o realce
   do Markdown;
2. fechar a janela logo depois de digitar: o texto tem de chegar ao disco
   antes de a janela fechar.
3. exportar logo depois de digitar: o capítulo é salvo antes, e o .docx sai
   com os capítulos em ordem e o estilo do livro.
4. tema escuro: a folha fica mais escura que a tinta.
5. modo foco: somem a lateral e as barras, e os avisos continuam na tela.
6. a escolha de tema fica guardada: clica no botão, fecha, abre de novo.
7. criar o estilo.yaml: numa pasta sem ele, o botão do aviso grava o modelo,
   a tela passa a usá-lo e o que foi digitado não se perde.
8. conforto: Ctrl+= e Ctrl+rolagem mudam o zoom da janela, F9 esconde a
   lateral, F10 leva a linha do cursor para o meio; ao abrir de novo, as
   três escolhas voltam.
9. escrita: aspas e hifens digitados viram aspas curvas e travessão, o botão
   desliga a troca, e Ctrl+F abre a busca em português e acha a palavra.
10. sessão: define a meta do dia, troca de capítulo e fecha; aberto de novo
    sem dizer a pasta, o app volta ao mesmo livro, capítulo e cursor, com a
    conta de palavras de hoje.

O app roda com a pasta de dados dele num temporário: o teste não lê nem
muda a escolha de tema de quem usa o programa nesta máquina. Por isso as
fontes instaladas só para o usuário (`~/.local/share/fonts`) também ficam de
fora: o teste só enxerga as fontes do sistema.

Não roda no CI. Sem `xvfb-run` instalado ele precisa de `--na-tela`. No
display virtual a captura usa o `import` do ImageMagick; na tela de verdade
só acontece no Hyprland e pega apenas o retângulo da janela do app.
"""

import json
import os
import pathlib
import queue
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import zipfile

LIMITE_SEGUNDOS = 60
FRASE = " Frase digitada pelo teste funcional."
# O salvamento automático espera 1,5 s. Fechar tem de salvar antes disso,
# senão quem salvou foi o temporizador e a passada 2 não provou nada.
PRAZO_PARA_FECHAR_SEGUNDOS = 1.3
RAIZ = pathlib.Path(__file__).resolve().parent.parent
ABERTURA = (
    "# Capítulo um\r\n\r\n"
    '::: {custom-style="Epígrafe"}\r\n'
    "A pele guarda o que a memória solta.\r\n"
    ":::\r\n\r\n"
    "Um parágrafo com *ênfase*, com **força** e com ~~um corte~~.\r\n\r\n"
)
PARAGRAFO = (
    "A casa ficava no alto do morro e de lá se via o rio inteiro quando a "
    "neblina deixava. Ela abria as janelas antes do café e esperava o barulho "
    "dos barcos que desciam carregados de sal e de farinha.\r\n\r\n"
)
# No fim do capítulo, longe da captura: um título colado num parágrafo, que
# para o Pandoc é texto do parágrafo e na tela não pode ficar com cara de título.
COLADO = "Um parágrafo com um título colado.\r\n# Para o Pandoc isto não é título\r\n\r\n"
SEGUNDO_CAPITULO = "# Capítulo dois\n\nOutro capítulo.\n".encode()
PALAVRAS_DA_FRASE = 5
# Um estilo com fonte que não existe: o app avisa, e o aviso tem de continuar
# na tela no modo foco.
ESTILO_COM_FONTE_AUSENTE = "\n  Placa:\n    fonte: Fonte Que Nao Existe 0000\n"
FORA_DA_TELA = os.environ.get("MD_NOVEL_TESTE_FORA_DA_TELA") == "1"
TEM_HYPRLAND = (
    shutil.which("hyprctl") is not None
    and shutil.which("grim") is not None
    and "HYPRLAND_INSTANCE_SIGNATURE" in os.environ
)


def contar_palavras(texto: str) -> int:
    """A regra de `crates/workspace/src/texto.rs`, refeita aqui para conferir
    o número que aparece na tela sem perguntar ao próprio app."""
    return sum(
        1
        for linha in texto.splitlines()
        if not linha.lstrip().startswith(":::")
        for trecho in linha.split()
        if any(caractere.isalnum() for caractere in trecho)
    )


def inteiro(texto: str) -> int:
    return int(texto.replace(".", ""))


def claridade(linha: str, nome: str) -> float | None:
    """A claridade (0 a 255) da cor `nome=rgb(r, g, b)` numa linha da interface."""
    achado = re.search(rf"{nome}=rgba?\((\d+), (\d+), (\d+)", linha)
    if achado is None:
        return None
    vermelho, verde, azul = (int(valor) for valor in achado.groups())
    return 0.2126 * vermelho + 0.7152 * verde + 0.0722 * azul


def montar_livro(pasta: pathlib.Path, estilo_a_mais: str = "") -> bytes:
    estilo = (RAIZ / "exemplos" / "estilo.yaml").read_text(encoding="utf-8")
    (pasta / "estilo.yaml").write_text(estilo + estilo_a_mais, encoding="utf-8")
    # CR LF de propósito: salvar tem de manter o formato do arquivo.
    primeiro = (ABERTURA + PARAGRAFO * 60 + COLADO).encode()
    (pasta / "01-abertura.md").write_bytes(primeiro)
    (pasta / "02-a-casa.md").write_bytes(SEGUNDO_CAPITULO)
    return primeiro


def hyprctl(*argumentos: str):
    return json.loads(subprocess.run(["hyprctl", *argumentos, "-j"], capture_output=True, text=True, check=True).stdout)


def janela_do_app(pid: int) -> dict | None:
    return next((cliente for cliente in hyprctl("clients") if cliente.get("pid") == pid), None)


def capturar(pid: int, destino: str) -> str:
    janela = janela_do_app(pid)
    visiveis = {monitor["activeWorkspace"]["id"] for monitor in hyprctl("monitors")}
    if janela is None or janela.get("workspace", {}).get("id") not in visiveis:
        return "captura: a janela não está numa área de trabalho visível; nada foi fotografado"
    (x, y), (largura, altura) = janela["at"], janela["size"]
    subprocess.run(["grim", "-g", f"{x},{y} {largura}x{altura}", destino], check=True)
    return f"captura: {largura}x{altura} em {destino}"


class App:
    """O binário rodando numa pasta de livro, com a saída lida numa thread:
    um app que não imprime nada não segura o teste além do limite."""

    def __init__(
        self,
        binario: str,
        pasta: pathlib.Path | None,
        fechar: bool = False,
        exportar_para: pathlib.Path | None = None,
        tema: str | None = "claro",
        foco: bool = False,
        cliques_no_tema: int = 0,
        dados: pathlib.Path | None = None,
        criar_estilo: bool = False,
        conforto: bool = False,
        escrita: bool = False,
        sessao: bool = False,
    ):
        # A pasta de dados do app (onde o WebKit guarda a escolha de tema e o
        # cache) vai para um temporário: a de quem usa o programa não é tocada.
        # `dados` reaproveita a de outra janela, como numa segunda abertura.
        self._dados = None if dados is not None else tempfile.TemporaryDirectory(prefix="md-novel-dados-")
        dados = dados if dados is not None else pathlib.Path(self._dados.name)
        ambiente = {
            **os.environ,
            "MD_NOVEL_TESTE_DIGITAR": FRASE,
            "MD_NOVEL_TESTE_CLIQUES_NO_TEMA": str(cliques_no_tema),
            "XDG_DATA_HOME": str(dados / "dados"),
            "XDG_CACHE_HOME": str(dados / "cache"),
        }
        # Sem tema no roteiro vale a escolha guardada, como para o autor.
        ambiente.pop("MD_NOVEL_TESTE_TEMA", None)
        if tema is not None:
            ambiente["MD_NOVEL_TESTE_TEMA"] = tema
        if fechar:
            ambiente["MD_NOVEL_TESTE_FECHAR"] = "1"
        if exportar_para is not None:
            ambiente["MD_NOVEL_TESTE_EXPORTAR"] = str(exportar_para)
        if foco:
            ambiente["MD_NOVEL_TESTE_FOCO"] = "1"
        if criar_estilo:
            ambiente["MD_NOVEL_TESTE_CRIAR_ESTILO"] = "1"
        if conforto:
            ambiente["MD_NOVEL_TESTE_CONFORTO"] = "1"
        if escrita:
            ambiente["MD_NOVEL_TESTE_ESCRITA"] = "1"
        if sessao:
            ambiente["MD_NOVEL_TESTE_SESSAO"] = "1"
        self.processo = subprocess.Popen(
            # Sem pasta: o app abre o último livro, como pelo menu.
            [binario] if pasta is None else [binario, str(pasta)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=ambiente
        )
        self.linhas: list[str] = []
        self._fila: queue.Queue = queue.Queue()
        threading.Thread(target=self._ler, daemon=True).start()

    def _ler(self) -> None:
        assert self.processo.stdout is not None
        for linha in self.processo.stdout:
            self._fila.put(linha)
        self._fila.put(None)

    def esperar_linha(self, trecho: str) -> str | None:
        """Lê até uma linha da interface com o trecho. `None` no limite ou se o app fechar."""
        prazo = time.monotonic() + LIMITE_SEGUNDOS
        while True:
            try:
                linha = self._fila.get(timeout=max(0.0, prazo - time.monotonic()))
            except queue.Empty:
                return None
            if linha is None:
                return None
            self.linhas.append(linha.rstrip())
            print("  " + linha.rstrip())
            if linha.startswith("[interface]") and trecho in linha:
                return linha

    def linha_com(self, prefixo: str) -> str:
        return next((linha for linha in self.linhas if linha.startswith(f"[interface] {prefixo}")), "")

    def encerrar(self) -> None:
        self.processo.terminate()
        try:
            self.processo.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.processo.kill()
            self.processo.wait()
        if self._dados is not None:
            self._dados.cleanup()


def conferir_disco(pasta: pathlib.Path, original: bytes, falhas: list[str], passada: str) -> None:
    gravado = (pasta / "01-abertura.md").read_bytes()
    if gravado != original + FRASE.encode():
        falhas.append(f"{passada}: o capítulo no disco não é o original mais a frase digitada")
    if b"\n" in gravado.replace(b"\r\n", b""):
        falhas.append(f"{passada}: o CR LF do arquivo não foi mantido")
    if (pasta / "02-a-casa.md").read_bytes() != SEGUNDO_CAPITULO:
        falhas.append(f"{passada}: o capítulo 2 mudou")
    sobras = sorted(entrada.name for entrada in pasta.iterdir())
    if sobras != ["01-abertura.md", "02-a-casa.md", "estilo.yaml"]:
        falhas.append(f"{passada}: arquivos na pasta do livro: {sobras}")


def passada_do_salvamento_automatico(binario: str, falhas: list[str]) -> None:
    print("passada 1: salvamento automático")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        original = montar_livro(pasta)
        app = App(binario, pasta)
        try:
            # O roteiro termina (linha "foco:") antes de o salvamento
            # automático disparar, 1,5 s depois de digitar.
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 1: a interface não chegou ao fim do roteiro")
            elif app.esperar_linha("salvo:") is None:
                falhas.append("passada 1: o app não chegou a salvar")
        finally:
            app.encerrar()

        def exigir(condicao: bool, falha: str) -> None:
            if not condicao:
                falhas.append(f"passada 1: {falha}")

        exigir("2 capítulos, css=true" in app.linha_com("livro:"), f"livro: {app.linha_com('livro:')!r}")
        exigir("01-abertura.md" in app.linha_com("capítulo:"), f"capítulo: {app.linha_com('capítulo:')!r}")
        corpo, nomeado = app.linha_com("linha do corpo:"), app.linha_com("linha de estilo nomeado:")
        exigir("align=justify" in corpo and "indent=47.2" in corpo, f"corpo: {corpo!r}")
        exigir(
            "align=right" in nomeado and "style=italic" in nomeado and "indent=0px" in nomeado,
            f"nomeado: {nomeado!r}",
        )
        rolagem = app.linha_com("rolagem:").removeprefix("[interface] rolagem: área ").replace("px", "").split(" de ")
        exigir(len(rolagem) == 2 and int(rolagem[0]) < int(rolagem[1]), f"o editor não rola por dentro: {rolagem}")

        # A lateral e a barra de status: título, arquivo e palavras do capítulo
        # aberto, antes e depois de digitar, contra a conta feita aqui.
        do_capitulo = contar_palavras(original.decode())
        do_livro = do_capitulo + contar_palavras(SEGUNDO_CAPITULO.decode())
        for quando, a_mais in (("antes", 0), ("depois", PALAVRAS_DA_FRASE)):
            linha = app.linha_com(f"contagem {quando}:")
            achado = re.search(
                r'capítulo="Capítulo: ([\d.]+) palavras" livro="Livro: ([\d.]+) palavras" '
                r'resumo="2 capítulos · ([\d.]+) palavras" item="Capítulo um \| ([\d.]+) \| 01-abertura.md"',
                linha,
            )
            exigir(achado is not None, f"contagem {quando}: {linha!r}")
            if achado is not None:
                visto = [inteiro(numero) for numero in achado.groups()]
                esperado = [do_capitulo + a_mais, do_livro + a_mais, do_livro + a_mais, do_capitulo + a_mais]
                exigir(visto == esperado, f"contagem {quando}: na tela {visto}, esperado {esperado}")

        tema = app.linha_com("tema:")
        papel, tinta = claridade(tema, "papel"), claridade(tema, "tinta")
        exigir("tema: claro" in tema and 'botão="Tema: claro"' in tema, f"tema: {tema!r}")
        exigir(papel is not None and tinta is not None and papel > tinta, f"no tema claro o papel é mais claro que a tinta: {tema!r}")

        realce = app.linha_com("realce:")
        tamanhos = re.search(r"título=([\d.]+)px corpo=([\d.]+)px", realce)
        exigir("ênfase=italic" in realce and "forte=700" in realce, f"realce: {realce!r}")
        exigir(
            tamanhos is not None and float(tamanhos.group(1)) > float(tamanhos.group(2)),
            f"o título não ficou maior que o corpo: {realce!r}",
        )
        # O analisador do editor segue o CommonMark e vê um título no `#`
        # colado ao parágrafo; a tela tem de mostrá-lo como o Pandoc: texto.
        colado = re.search(r"texto=([\d.]+)px corpo=([\d.]+)px", app.linha_com("título colado:"))
        exigir(
            colado is not None and colado.group(1) == colado.group(2),
            f"o título colado no parágrafo ficou com tamanho de título: {app.linha_com('título colado:')!r}",
        )
        sinal = claridade(realce, "sinal")
        exigir(
            sinal is not None and tinta is not None and sinal > tinta,
            f"o sinal do Markdown não ficou mais apagado que a tinta: {realce!r} / {tema!r}",
        )

        foco = app.linha_com("foco:")
        exigir(
            "foco: desligado" in foco
            and all(f"{parte}=flex" in foco for parte in ("lateral", "barra", "status"))
            and "sair-do-foco=none" in foco,
            f"foco: {foco!r}",
        )
        conferir_disco(pasta, original, falhas, "passada 1")


def passada_de_fechar_a_janela(binario: str, falhas: list[str]) -> None:
    print("passada 2: fechar a janela logo depois de digitar")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        original = montar_livro(pasta)
        # O app pede o próprio fechamento logo depois de digitar, pelo mesmo
        # evento que o gerenciador de janelas dispara.
        app = App(binario, pasta, fechar=True)
        try:
            if app.esperar_linha("digitado pelo roteiro de teste") is None:
                falhas.append("passada 2: a interface não digitou")
                return
            digitado = time.monotonic()
            try:
                app.processo.wait(timeout=10)
            except subprocess.TimeoutExpired:
                falhas.append("passada 2: a janela não fechou depois de salvar")
                return
            demora = time.monotonic() - digitado
            print(f"  fechou {demora:.2f}s depois de digitar")
            if demora >= PRAZO_PARA_FECHAR_SEGUNDOS:
                falhas.append(f"passada 2: fechou em {demora:.2f}s; o salvamento automático pode ter salvo antes")
        finally:
            app.encerrar()
        conferir_disco(pasta, original, falhas, "passada 2")


def capturar_o_display_virtual(destino: str) -> str:
    """No display virtual a janela do app é a única coisa que existe."""
    if shutil.which("import") is None:
        return "captura: sem o `import` do ImageMagick, nada foi fotografado"
    subprocess.run(["import", "-window", "root", destino], check=True)
    return f"captura: display virtual em {destino}"


def passada_de_exportar(binario: str, captura: str | None, falhas: list[str]) -> None:
    print("passada 3: exportar para .docx logo depois de digitar")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria, tempfile.TemporaryDirectory(
        prefix="md-novel-saida-"
    ) as saida:
        pasta = pathlib.Path(temporaria)
        original = montar_livro(pasta)
        destino = pathlib.Path(saida) / "livro.docx"
        # O destino vem por variável de ambiente no lugar do seletor do sistema.
        app = App(binario, pasta, exportar_para=destino)
        try:
            if app.esperar_linha("exportado:") is None:
                falhas.append("passada 3: o app não chegou a exportar")
                return
            # Depois de exportar a janela mostra o aviso de livro exportado.
            capturar_se_pedido(app, captura, "")
        finally:
            app.encerrar()
        conferir_disco(pasta, original, falhas, "passada 3")
        if not destino.exists():
            falhas.append("passada 3: o .docx não apareceu no destino")
            return
        with zipfile.ZipFile(destino) as docx:
            documento = docx.read("word/document.xml").decode()
            estilos = docx.read("word/styles.xml").decode()
        # A frase foi digitada um instante antes: se está no .docx, o app
        # salvou o capítulo antes de exportar.
        if FRASE.strip() not in documento:
            falhas.append("passada 3: o .docx não tem a frase digitada; o capítulo não foi salvo antes de exportar")
        if '<w:pStyle w:val="Epígrafe"' not in documento:
            falhas.append("passada 3: a epígrafe não saiu com o estilo nomeado")
        if "Capítulo dois" not in documento or documento.find("Capítulo um") > documento.find("Capítulo dois"):
            falhas.append("passada 3: os capítulos não saíram em ordem")
        if '<w:jc w:val="both"' not in estilos:
            falhas.append("passada 3: o corpo não saiu justificado como manda o estilo.yaml")
        if "0 avisos" not in app.linha_com("exportado:"):
            falhas.append(f"passada 3: {app.linha_com('exportado:')!r}")


def capturar_se_pedido(app: "App", captura: str | None, sufixo: str) -> None:
    if not captura:
        return
    destino = pathlib.Path(captura)
    destino = destino.with_name(f"{destino.stem}{sufixo}{destino.suffix}")
    time.sleep(1)
    if FORA_DA_TELA:
        print("  " + capturar_o_display_virtual(str(destino)))
    elif TEM_HYPRLAND:
        print("  " + capturar(app.processo.pid, str(destino)))


def passada_do_tema_escuro(binario: str, captura: str | None, falhas: list[str]) -> None:
    print("passada 4: tema escuro")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        montar_livro(pasta)
        app = App(binario, pasta, tema="escuro")
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 4: a interface não chegou ao fim do roteiro")
                return
            capturar_se_pedido(app, captura, "-escuro")
        finally:
            app.encerrar()
        tema = app.linha_com("tema:")
        mesa, papel, tinta = (claridade(tema, nome) for nome in ("mesa", "papel", "tinta"))
        if "tema: escuro" not in tema or 'botão="Tema: escuro"' not in tema:
            falhas.append(f"passada 4: {tema!r}")
        if mesa is None or papel is None or tinta is None or not (mesa < papel < tinta):
            falhas.append(f"passada 4: no tema escuro a mesa é mais escura que o papel, e o papel que a tinta: {tema!r}")


def passada_do_modo_foco(binario: str, captura: str | None, falhas: list[str]) -> None:
    print("passada 5: modo foco")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        montar_livro(pasta, ESTILO_COM_FONTE_AUSENTE)
        app = App(binario, pasta, foco=True)
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 5: a interface não chegou ao fim do roteiro")
                return
            capturar_se_pedido(app, captura, "-foco")
        finally:
            app.encerrar()
        foco = app.linha_com("foco:")
        if "foco: ligado" not in foco or any(f"{parte}=none" not in foco for parte in ("lateral", "barra", "status")):
            falhas.append(f"passada 5: a lateral e as barras não sumiram: {foco!r}")
        # O aviso da fonte que falta continua na tela, e há como sair do foco.
        if "avisos=none" in foco or "sair-do-foco=none" in foco:
            falhas.append(f"passada 5: o aviso ou o botão de sair do foco não estão na tela: {foco!r}")


def passada_da_escolha_de_tema(binario: str, falhas: list[str]) -> None:
    print("passada 6: a escolha de tema fica guardada")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria, tempfile.TemporaryDirectory(
        prefix="md-novel-dados-"
    ) as dados:
        pasta = pathlib.Path(temporaria)
        montar_livro(pasta)
        # Primeira janela: dois cliques no botão (sistema, claro, escuro) e
        # fecha em seguida, como o autor fecharia.
        app = App(binario, pasta, tema=None, cliques_no_tema=2, fechar=True, dados=pathlib.Path(dados))
        try:
            if app.esperar_linha("digitado pelo roteiro de teste") is None:
                falhas.append("passada 6: a primeira janela não chegou ao fim do roteiro")
                return
            try:
                app.processo.wait(timeout=10)
            except subprocess.TimeoutExpired:
                falhas.append("passada 6: a primeira janela não fechou")
                return
        finally:
            app.encerrar()
        if 'tema: escuro; botão="Tema: escuro"' not in app.linha_com("tema:"):
            falhas.append(f"passada 6: os dois cliques não levaram ao tema escuro: {app.linha_com('tema:')!r}")
        # Segunda janela, na mesma pasta de dados e sem tema no roteiro.
        app = App(binario, pasta, tema=None, dados=pathlib.Path(dados))
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 6: a segunda janela não chegou ao fim do roteiro")
                return
        finally:
            app.encerrar()
        if 'tema: escuro; botão="Tema: escuro"' not in app.linha_com("tema:"):
            falhas.append(f"passada 6: a escolha não ficou guardada: {app.linha_com('tema:')!r}")


def passada_de_criar_o_estilo(binario: str, falhas: list[str]) -> None:
    print("passada 7: criar o estilo.yaml pelo botão do aviso")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        original = montar_livro(pasta)
        modelo = (pasta / "estilo.yaml").read_bytes()
        (pasta / "estilo.yaml").unlink()
        app = App(binario, pasta, criar_estilo=True)
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 7: a interface não chegou ao fim do roteiro")
                return
            if app.esperar_linha("salvo:") is None:
                falhas.append("passada 7: o app não chegou a salvar")
                return
        finally:
            app.encerrar()
        antes = app.linha_com("linha do corpo:")
        botao = app.linha_com("botão do estilo:")
        depois = app.linha_com("depois de criar o estilo:")
        if "css=false" not in app.linha_com("livro:") or "align=justify" in antes:
            falhas.append(f"passada 7: a pasta sem estilo já abriu com estilo: {antes!r}")
        if 'botão do estilo: "Criar estilo.yaml"' not in botao:
            falhas.append(f"passada 7: o aviso não tem o botão de criar o estilo: {botao!r}")
        # O corpo do modelo é justificado; sai o aviso com o botão e entra o de "feito".
        if "align=justify" not in depois or "feito" not in depois:
            falhas.append(f"passada 7: a tela não passou a usar o estilo criado: {depois!r}")
        gravado = pasta / "estilo.yaml"
        if not gravado.is_file() or gravado.read_bytes() != modelo:
            falhas.append("passada 7: o estilo.yaml gravado não é o exemplos/estilo.yaml")
        # O capítulo com a frase digitada, o outro intacto e mais nada na pasta.
        conferir_disco(pasta, original, falhas, "passada 7")


def passada_do_conforto(binario: str, falhas: list[str]) -> None:
    print("passada 8: zoom, lateral e máquina de escrever, e as escolhas guardadas")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria, tempfile.TemporaryDirectory(
        prefix="md-novel-dados-"
    ) as dados:
        pasta = pathlib.Path(temporaria)
        montar_livro(pasta)
        # Primeira janela: usa as teclas e fecha, como o autor fecharia.
        app = App(binario, pasta, conforto=True, fechar=True, dados=pathlib.Path(dados))
        try:
            if app.esperar_linha("tela cheia:") is None:
                falhas.append("passada 8: a primeira janela não chegou ao fim do roteiro")
                return
            try:
                app.processo.wait(timeout=10)
            except subprocess.TimeoutExpired:
                falhas.append("passada 8: a primeira janela não fechou")
                return
        finally:
            app.encerrar()
        abertura = app.linha_com("conforto ao abrir:")
        if "zoom=100%" not in abertura or "lateral=flex" not in abertura or "máquina=desligada" not in abertura:
            falhas.append(f"passada 8: a janela nova não abriu no padrão: {abertura!r}")
        # Dois Ctrl+= e uma rolagem para baixo: 100, 110, 120, 110.
        zoom = app.linha_com("zoom:")
        larguras = re.search(r"largura (\d+) -> (\d+)", zoom)
        if 'botão="110%"' not in zoom or larguras is None or not int(larguras[2]) < int(larguras[1]):
            falhas.append(f"passada 8: o zoom não chegou a 110% ou a janela não aproximou: {zoom!r}")
        if "lateral: none apertado=true" not in app.linha_com("lateral:"):
            falhas.append(f"passada 8: F9 não escondeu a lateral: {app.linha_com('lateral:')!r}")
        maquina = app.linha_com("máquina:")
        desvios = re.search(r"antes=(\d+)px de (\d+)px depois=(\d+)px", maquina)
        # Antes o cursor está no fim da tela; depois, a menos de duas linhas do meio.
        if "máquina: ligada" not in maquina or desvios is None or not (
            int(desvios[3]) < 60 and int(desvios[1]) > int(desvios[2]) / 4
        ):
            falhas.append(f"passada 8: F10 não levou a linha do cursor para o meio: {maquina!r}")
        if "pedido aceito (true e depois false)" not in app.linha_com("tela cheia:"):
            falhas.append(f"passada 8: {app.linha_com('tela cheia:')!r}")
        # Segunda janela, na mesma pasta de dados e sem tocar em nada.
        app = App(binario, pasta, dados=pathlib.Path(dados))
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 8: a segunda janela não chegou ao fim do roteiro")
                return
        finally:
            app.encerrar()
        abertura = app.linha_com("conforto ao abrir:")
        if "zoom=110%" not in abertura or "lateral=none" not in abertura or "máquina=ligada" not in abertura:
            falhas.append(f"passada 8: as escolhas não ficaram guardadas: {abertura!r}")
        reaberta = re.search(r"largura=(\d+)", abertura)
        if larguras is not None and (reaberta is None or int(reaberta[1]) != int(larguras[2])):
            falhas.append(f"passada 8: o zoom guardado não foi aplicado ao abrir: {abertura!r}")


def passada_da_escrita(binario: str, falhas: list[str]) -> None:
    print("passada 9: aspas, travessão e busca")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria:
        pasta = pathlib.Path(temporaria)
        montar_livro(pasta)
        app = App(binario, pasta, escrita=True)
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 9: a interface não chegou ao fim do roteiro")
                return
        finally:
            app.encerrar()
        ligada = app.linha_com("tipografia: ligada")
        desligada = app.linha_com("tipografia: desligada")
        if "fim=[ “oi” — d’]" not in ligada:
            falhas.append(f"passada 9: as aspas e o travessão não entraram ao digitar: {ligada!r}")
        if 'fim=[ "x" --] apertado=false' not in desligada:
            falhas.append(f"passada 9: com o botão desligado o sinal não ficou reto: {desligada!r}")
        if 'fim=["c" --]' not in app.linha_com("tipografia: em código"):
            falhas.append(f"passada 9: dentro de um bloco de código o sinal foi trocado: {app.linha_com('tipografia: em código')!r}")
        painel = app.linha_com("busca: painel")
        if "painel=true" not in painel or 'campo="Buscar"' not in painel or "substituir todos" not in painel:
            falhas.append(f"passada 9: Ctrl+F não abriu a busca em português: {painel!r}")
        if 'busca: achou="neblina"' not in app.linha_com("busca: achou"):
            falhas.append(f"passada 9: a busca não selecionou a palavra: {app.linha_com('busca: achou')!r}")


def passada_da_sessao(binario: str, falhas: list[str]) -> None:
    print("passada 10: reabrir onde parou e a meta do dia")
    with tempfile.TemporaryDirectory(prefix="md-novel-teste-") as temporaria, tempfile.TemporaryDirectory(
        prefix="md-novel-dados-"
    ) as dados:
        pasta = pathlib.Path(temporaria)
        montar_livro(pasta)
        # Primeira janela: meta de 50, segundo capítulo, cursor na posição 7, fecha.
        app = App(binario, pasta, sessao=True, fechar=True, dados=pathlib.Path(dados))
        try:
            if app.esperar_linha("onde guardado:") is None:
                falhas.append("passada 10: a primeira janela não chegou ao fim do roteiro")
                return
            try:
                app.processo.wait(timeout=10)
            except subprocess.TimeoutExpired:
                falhas.append("passada 10: a primeira janela não fechou")
                return
        finally:
            app.encerrar()
        abertura = app.linha_com("onde:")
        if 'capítulo=01-abertura.md posição=0 hoje="Hoje: 0 palavras"' not in abertura:
            falhas.append(f"passada 10: o livro novo não abriu no começo, com a conta zerada: {abertura!r}")
        if f'hoje: "Hoje: {PALAVRAS_DA_FRASE} de 50 palavras" campo=fechado' not in app.linha_com("hoje:"):
            falhas.append(f"passada 10: a meta não entrou ou a conta de hoje está errada: {app.linha_com('hoje:')!r}")
        if "onde guardado: capítulo=02-a-casa.md posição=7" not in app.linha_com("onde guardado:"):
            falhas.append(f"passada 10: {app.linha_com('onde guardado:')!r}")
        # Segunda janela: sem pasta na linha de comando, como pelo menu.
        app = App(binario, None, dados=pathlib.Path(dados))
        try:
            if app.esperar_linha("foco:") is None:
                falhas.append("passada 10: sem pasta, o app não reabriu o último livro")
                return
        finally:
            app.encerrar()
        esperado = f'onde: capítulo=02-a-casa.md posição=7 hoje="Hoje: {PALAVRAS_DA_FRASE} de 50 palavras"'
        if esperado not in app.linha_com("onde:"):
            falhas.append(f"passada 10: não voltou ao capítulo, ao cursor e à conta de hoje: {app.linha_com('onde:')!r}")
        sobras = sorted(entrada.name for entrada in pasta.iterdir())
        if sobras != ["01-abertura.md", "02-a-casa.md", "estilo.yaml"]:
            falhas.append(f"passada 10: arquivos na pasta do livro: {sobras}")


def reexecutar_fora_da_tela(argumentos: list[str]) -> int:
    """Roda este mesmo teste dentro de um display virtual. O WebKitGTK usa o
    X11 dele, com desenho por software, e nada aparece na tela."""
    ambiente = {
        chave: valor
        for chave, valor in os.environ.items()
        if chave not in ("WAYLAND_DISPLAY", "HYPRLAND_INSTANCE_SIGNATURE")
    }
    ambiente.update(
        GDK_BACKEND="x11",
        LIBGL_ALWAYS_SOFTWARE="1",
        WEBKIT_DISABLE_DMABUF_RENDERER="1",
        MD_NOVEL_TESTE_FORA_DA_TELA="1",
    )
    comando = ["xvfb-run", "--auto-servernum", "--server-args=-screen 0 1200x900x24"]
    # As opções passam como estão; binário e png viram caminho inteiro.
    absolutos = [
        argumento if argumento.startswith("--") else str(pathlib.Path(argumento).resolve())
        for argumento in argumentos
    ]
    comando += [sys.executable, "-I", "-B", str(pathlib.Path(__file__).resolve()), *absolutos]
    return subprocess.run(comando, env=ambiente, check=False).returncode


def principal() -> int:
    opcoes = [argumento for argumento in sys.argv[1:] if argumento.startswith("--")]
    argumentos = [argumento for argumento in sys.argv[1:] if not argumento.startswith("--")]
    so_a_passada = None
    for opcao in opcoes:
        escolhida = re.fullmatch(r"--passada=([1-9]|10)", opcao)
        if escolhida is not None:
            so_a_passada = int(escolhida[1])
        elif opcao != "--na-tela":
            print(f"opção desconhecida: {opcao!r}; as opções são --na-tela e --passada=N, com N de 1 a 10")
            return 2
    na_tela = "--na-tela" in sys.argv[1:]
    if not argumentos:
        print(__doc__)
        return 2
    if not na_tela and not FORA_DA_TELA:
        if shutil.which("xvfb-run") is None:
            print("sem xvfb-run: instale o Xvfb ou passe --na-tela para abrir a janela na sua tela")
            return 2
        return reexecutar_fora_da_tela([*opcoes, *argumentos])
    binario = argumentos[0]
    captura = argumentos[1] if len(argumentos) > 1 else None
    falhas: list[str] = []
    passadas = [
        lambda: passada_do_salvamento_automatico(binario, falhas),
        lambda: passada_de_fechar_a_janela(binario, falhas),
        lambda: passada_de_exportar(binario, captura, falhas),
        lambda: passada_do_tema_escuro(binario, captura, falhas),
        lambda: passada_do_modo_foco(binario, captura, falhas),
        lambda: passada_da_escolha_de_tema(binario, falhas),
        lambda: passada_de_criar_o_estilo(binario, falhas),
        lambda: passada_do_conforto(binario, falhas),
        lambda: passada_da_escrita(binario, falhas),
        lambda: passada_da_sessao(binario, falhas),
    ]
    for numero, passada in enumerate(passadas, start=1):
        if so_a_passada is None or so_a_passada == numero:
            passada()
    for falha in falhas:
        print(f"FALHOU: {falha}")
    resultado = "reprovado" if falhas else "aprovado"
    print("teste funcional:", resultado if so_a_passada is None else f"{resultado} (só a passada {so_a_passada})")
    return 1 if falhas else 0


if __name__ == "__main__":
    sys.exit(principal())
