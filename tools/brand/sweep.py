#!/usr/bin/env python3
"""Barrido de marca: busca restos de la marca original en el repo.

Solo biblioteca estándar. Uso:
    python3 tools/brand/sweep.py [--root .] [--out docs/brand/sweep-report.md] [--strict]

Clasifica cada coincidencia en:
  PERMITIDA  uso técnico/documental legítimo (nombre de crate, protocolo, URL de docs, docs/).
  REVISAR    aparece en código o recursos de la app (puede acabar en pantalla): hay que mirarla.
  BLOQUEANTE texto visible al usuario (cadenas de UI, assets, README de cara al público, metadatos).
Con --strict sale con código 1 si hay REVISAR o BLOQUEANTE.
"""
import argparse, os, re, sys, collections, datetime

TERMS = re.compile(r"telegram|tdesktop|durov|\bt\.me\b|\btg://", re.I)
SKIP_DIRS = {".git", "target", ".venv", "graphify-out", "node_modules", "__pycache__", ".claude"}
TEXT_EXT = {".rs", ".toml", ".md", ".json", ".csv", ".py", ".txt", ".yml", ".yaml", ".svg",
            ".desktop", ".ron", ".ftl", ".html", ".css", ".sh", ".lock", ".xml", ""}
# Rutas donde mencionar la marca original es documentación legítima (clean-room, referencias).
DOC_PREFIXES = ("docs/", "replica/", "CLAUDE.md", "tools/brand/sweep.py")
# Crates cuyo nombre y código interno referencian el protocolo/servicio por necesidad técnica.
TECH_PREFIXES = ("crates/tc-telegram/", "crates/tc-secret/", "crates/tc-core/", "crates/tc-store/")
# Fragmentos técnicos permitidos en cualquier sitio.
ALLOWED_FRAGMENTS = re.compile(
    r"tc[-_]telegram|core\.telegram\.org|telegram\.org/(faq|api|support|blog)|desktop\.telegram\.org|"
    r"tg://login|grammers|mtproto|"
    # Valores e identificadores técnicos del selector de backend (no son nombre de producto; ADR-0005):
    r"TC_BACKEND=telegram|Some\(\"telegram\"\)|«telegram»|`telegram`|my\.telegram\.org|"
    r"\.join\(\"telegram(/session\.json)?\"\)|Choice::Telegram|\bTelegram\(TelegramConfig\)|"
    r"Telegram(Config|Paths|Stores)\b|telegram_(config|connector|stores|con_credenciales)\b|"
    r"`telegram_stores`|tc_telegram::|\.telegram/|/telegram/session\.json", re.I)
# Rutas de código de UI/app: lo que contenga aquí puede verse en pantalla.
UI_PREFIXES = ("crates/tc-ui/", "crates/tc-app/", "crates/tc-theme/", "crates/tc-mock/", "assets/")
PUBLIC_FILES = ("README.md",)
STRING_LITERAL = re.compile(r'"[^"\n]*(?:telegram|tdesktop|durov)[^"\n]*"', re.I)


def classify(rel, line):
    if rel in PUBLIC_FILES:
        return "PERMITIDA" if not TERMS.search(ALLOWED_FRAGMENTS.sub("", line)) else "BLOQUEANTE"
    if rel.startswith(DOC_PREFIXES):
        return "PERMITIDA"
    stripped = TERMS.sub("", ALLOWED_FRAGMENTS.sub("", line))
    only_allowed = not TERMS.search(ALLOWED_FRAGMENTS.sub("", line))
    if only_allowed:
        return "PERMITIDA"
    if rel.endswith("Cargo.toml") or rel == "Cargo.lock":
        # Comentarios y descripción del crate de red (tc-telegram) o dependencias: documentación técnica.
        if only_allowed or line.lstrip().startswith("#") or rel.startswith("crates/tc-telegram/"):
            return "PERMITIDA"
        return "REVISAR"
    if rel.startswith(UI_PREFIXES):
        if STRING_LITERAL.search(line) or rel.startswith("assets/"):
            return "BLOQUEANTE"
        return "REVISAR"
    if rel.startswith(TECH_PREFIXES):
        t = line.lstrip()
        # comentarios, identificadores o doc-comments en crates de red: uso técnico
        if t.startswith(("//", "///", "//!", "#", "*")) or not STRING_LITERAL.search(line):
            return "PERMITIDA"
        return "REVISAR"
    return "REVISAR"


def scan(root):
    hits = []
    for dp, dns, fns in os.walk(root):
        dns[:] = [d for d in dns if d not in SKIP_DIRS]
        for fn in fns:
            path = os.path.join(dp, fn)
            rel = os.path.relpath(path, root).replace(os.sep, "/")
            if TERMS.search(rel):
                hits.append((rel, 0, "(nombre de ruta)", "PERMITIDA" if rel.startswith(TECH_PREFIXES + DOC_PREFIXES) else "REVISAR"))
            ext = os.path.splitext(fn)[1].lower()
            if ext not in TEXT_EXT:
                continue
            try:
                with open(path, encoding="utf-8") as f:
                    for n, line in enumerate(f, 1):
                        if TERMS.search(line):
                            hits.append((rel, n, line.strip()[:160], classify(rel, line)))
            except (UnicodeDecodeError, OSError):
                continue
    return hits


def report(hits, root):
    by = collections.Counter(h[3] for h in hits)
    out = ["# Informe de barrido de marca", "",
           f"Generado: {datetime.date.today().isoformat()} por `tools/brand/sweep.py` (raíz `{root}`).", "",
           "Busca: telegram, tdesktop, durov, t.me, tg://. Excluye .git, target, .venv, graphify-out, .claude.", "",
           "## Resumen", "",
           "| Clase | Coincidencias |", "| --- | ---: |"]
    for k in ("BLOQUEANTE", "REVISAR", "PERMITIDA"):
        out.append(f"| {k} | {by.get(k, 0)} |")
    out.append("")
    for k, title in (("BLOQUEANTE", "Bloqueantes (texto visible o assets)"), ("REVISAR", "A revisar (código que podría llegar a pantalla)")):
        rows = [h for h in hits if h[3] == k]
        out += [f"## {title}", ""]
        if not rows:
            out += ["Ninguna.", ""]
            continue
        out += ["| Archivo | Línea | Texto |", "| --- | ---: | --- |"]
        for rel, n, txt, _ in rows:
            out.append(f"| `{rel}` | {n} | `{txt.replace('|', chr(92) + '|')}` |")
        out.append("")
    per = collections.Counter(h[0] for h in hits if h[3] == "PERMITIDA")
    out += ["## Permitidas (resumen por archivo)", "", "Nombres de crate, protocolo, URLs de documentación y docs de clean-room.", "",
            "| Archivo | Coincidencias |", "| --- | ---: |"]
    for rel, c in sorted(per.items(), key=lambda x: (-x[1], x[0]))[:40]:
        out.append(f"| `{rel}` | {c} |")
    if len(per) > 40:
        out.append(f"| ... {len(per) - 40} archivos más | |")
    out.append("")
    return "\n".join(out), by


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=".")
    ap.add_argument("--out", default=None)
    ap.add_argument("--strict", action="store_true")
    a = ap.parse_args()
    text, by = report(scan(a.root), a.root)
    if a.out:
        with open(a.out, "w", encoding="utf-8") as f:
            f.write(text)
    print(text if not a.out else f"Informe en {a.out}: " + ", ".join(f"{k}={v}" for k, v in by.items()))
    if a.strict and (by.get("BLOQUEANTE") or by.get("REVISAR")):
        sys.exit(1)


if __name__ == "__main__":
    main()
