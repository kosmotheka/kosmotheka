#!/usr/bin/env python3
"""Build the context packet an audit agent reads, so agents never explore the repository.

Usage:
  packet.py review <work> <topic.md> [--incremental]
  packet.py triage <work> <category,...> [--limit N] [--context N] [--incremental]

<work> is the audit folder made by prepare.sh (src/, facts/, tools/). Packets are
written to <work>/packets/. Prints each packet path with its size in lines.
Exit code 3 in incremental mode means the topic has no changes: skip it.
Runs on the host's python3 (3.8), standard library only.
"""
import fnmatch
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
FENCE = {".rs": "rust", ".ts": "ts", ".tsx": "tsx", ".php": "php", ".json": "json", ".sql": "sql",
         ".md": "markdown", ".yaml": "yaml", ".yml": "yaml", ".xml": "xml", ".toml": "toml"}


def read(path):
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


def outline(work):
    rows = []
    for line in read(work / "facts/outline.tsv").split("\n")[1:]:
        parts = line.split("\t")
        if len(parts) == 6:
            rows.append({"path": parts[0], "start": int(parts[1]), "end": int(parts[2]),
                         "name": parts[4], "flags": parts[5]})
    return rows


def changed_ranges(work):
    ranges, path = {}, None
    for line in read(work / "tools/diff.patch").split("\n"):
        if line.startswith("+++ "):
            path = None if line.endswith("/dev/null") else line[6:]
        m = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", line)
        if m and path:
            start = int(m.group(1))
            ranges.setdefault(path, []).append((start, start + max(int(m.group(2) or 1), 1) - 1))
    return ranges


def excerpt(src_lines, start, end, path, title=""):
    lang = FENCE.get(Path(path).suffix, "")
    body = [f"{n:5} {src_lines[n - 1]}" for n in range(max(start, 1), min(end, len(src_lines)) + 1)]
    return [f"### {path}:{start}-{end}{' · ' + title if title else ''}", f"```{lang}"] + body + ["```", ""]


def section(text, heading):
    """Lines under a Markdown heading, up to the next heading of the same or higher level."""
    lines, out, level = text.split("\n"), [], None
    for line in lines:
        m = re.match(r"^(#+) (.*)$", line)
        if level is None:
            if m and m.group(2).strip().lower().startswith(heading.lower()):
                level = len(m.group(1))
                out.append(line)
            continue
        if m and len(m.group(1)) <= level:
            break
        out.append(line)
    return out


def parse_topic(path):
    text = read(path)
    m = re.match(r"^---\n(.*?)\n---\n(.*)$", text, re.S)
    if not m:
        sys.exit(f"{path}: missing header")
    header, key = {}, None
    for line in m.group(1).split("\n"):
        if line.startswith("  - ") and key:
            header.setdefault(key, []).append(line[4:].split("  #")[0].strip())
        elif ":" in line:
            key, value = line.split(":", 1)
            key = key.strip()
            if value.strip():
                header[key] = value.split("  #")[0].strip()
    return header, m.group(2).strip()


def code_items(work, spec, rows, changes, incremental):
    snap = work / "src"
    blocks = []

    def add_file(rel):
        src = read(snap / rel).split("\n")
        if incremental:
            if rel not in changes:
                return
            funcs = [r for r in rows if r["path"] == rel and any(a <= r["end"] and r["start"] <= b for a, b in changes[rel])]
            for r in funcs:
                blocks.append((rel, r["start"], r["end"], r["name"]))
            covered = [(r["start"], r["end"]) for r in funcs]
            for a, b in changes[rel]:
                if not any(s <= a and b <= e for s, e in covered):
                    blocks.append((rel, max(1, a - 5), b + 5, "changed lines"))
            return
        blocks.append((rel, 1, len(src), ""))

    def pick(rel, pattern):
        blocks.extend(pick_functions(rows, rel, pattern, changes, incremental))

    for item in spec:
        if item.startswith("@longest-"):
            lang, n = item[len("@longest-"):].split(":")
            ext = (".rs",) if lang == "rust" else (".ts", ".tsx")
            cands = [r for r in rows if r["path"].endswith(ext) and "test" not in r["flags"]]
            for r in sorted(cands, key=lambda r: r["end"] - r["start"], reverse=True)[:int(n)]:
                if not incremental or any(a <= r["end"] and r["start"] <= b for a, b in changes.get(r["path"], [])):
                    blocks.append((r["path"], r["start"], r["end"], r["name"]))
            continue
        if "::" in item:
            path_part, pattern = item.split("::", 1)
            paths = sorted(p.relative_to(snap).as_posix() for p in snap.glob(path_part) if p.is_file()) if "*" in path_part else [path_part]
            for rel in paths:
                if pattern == "@prod":
                    if incremental:
                        add_file(rel)
                        continue
                    src = read(snap / rel).split("\n")
                    cut = next((i for i, l in enumerate(src) if l.strip() == "#[cfg(test)]"), len(src))
                    blocks.append((rel, 1, cut, "production code"))
                    continue
                pick(rel, pattern)
            continue
        matches = sorted(p.relative_to(snap).as_posix() for p in snap.glob(item) if p.is_file()) if "*" in item else [item]
        for rel in matches:
            add_file(rel)
    return render(blocks, snap)


def pick_functions(rows, rel, pattern, changes, incremental):
    want_tests = pattern == "@tests"
    out = []
    for r in rows:
        if r["path"] != rel:
            continue
        if want_tests != ("test" in r["flags"]):
            continue
        if not want_tests and not any(fnmatch.fnmatch(r["name"], p) for p in pattern.split("|")):
            continue
        if incremental and not any(a <= r["end"] and r["start"] <= b for a, b in changes.get(rel, [])):
            continue
        out.append((rel, r["start"], r["end"], r["name"]))
    return out


def render(blocks, snap):
    seen, out = set(), []
    for rel, a, b, title in sorted(blocks):
        if (rel, a, b) in seen:
            continue
        seen.add((rel, a, b))
        out += excerpt(read(snap / rel).split("\n"), a, b, rel, title)
    return out


def context_items(work, items):
    out = []
    for item in items:
        path, _, heading = item.partition("#")
        base = work if path.startswith("facts/") else work / "src"
        text = read(base / path)
        body = section(text, heading) if heading else text.split("\n")
        out += [f"### {item}", ""] + body + [""]
    return out


def decision_items(work, spec):
    text = read(work / "src/docs/DECISIONS.md")
    parts = re.split(r"(?m)^(?=## D\d+ · )", text)
    wanted = spec.split()
    out = []
    for part in parts:
        m = re.match(r"## (D\d+) · (.+)", part)
        if m and any(w == m.group(1) or w.lower() in m.group(2).lower() for w in wanted):
            out += part.rstrip().split("\n") + [""]
    return out


def commit(work):
    return read(work / "tools/commit.txt").strip()[:8]


def emit(work, name, lines):
    folder = work / "packets"
    folder.mkdir(exist_ok=True)
    path = folder / f"{name}.md"
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{path}\t{len(lines)} lines")


def review(work, topic_path, incremental):
    header, questions = parse_topic(topic_path)
    rows, changes = outline(work), changed_ranges(work) if incremental else {}
    code = code_items(work, header.get("code", []), rows, changes, incremental)
    if incremental and not code:
        print(f"{topic_path.stem}: no changes in its code")
        sys.exit(3)
    rules = read(HERE / "review-rules.md").split("\n")
    lines = [f"# Paquete de auditoría · {topic_path.stem} · {commit(work)}", "",
             f"- SNAPSHOT: {work / 'src'}", f"- Modo: {'incremental (solo el código que ha cambiado)' if incremental else 'completo'}", ""]
    lines += rules + ["", "## Preguntas de este tema", "", questions, "", "## Contexto", ""]
    lines += context_items(work, header.get("context", []))
    if header.get("decisions"):
        lines += ["### Decisiones relacionadas (docs/DECISIONS.md)", ""] + decision_items(work, header["decisions"])
    lines += ["## Código", ""] + code
    emit(work, f"review-{topic_path.stem}", lines)
    limit = int(header.get("max_lines", "2500"))
    if len(lines) > limit:
        print(f"WARNING: {len(lines)} lines is over max_lines {limit}; split the topic or narrow its code list")


def enclosing(rows, rel, ln, context, whole=60):
    """The whole function around a candidate when it is short, so the agent sees every
    caller-visible line; otherwise the lines around it."""
    inner = [r for r in rows if r["path"] == rel and r["start"] <= ln <= r["end"]]
    if inner:
        f = min(inner, key=lambda r: r["end"] - r["start"])
        if f["end"] - f["start"] < whole:
            return f["start"], f["end"]
    return ln - context, ln + context


def triage(work, categories, limit, context, incremental):
    patterns = categories.split(",")
    changed = set(read(work / "tools/changed.txt").split()) if incremental else None
    cands = []
    for line in read(work / "facts/candidates.tsv").split("\n")[1:]:
        parts = line.split("\t")
        if len(parts) != 4 or not any(fnmatch.fnmatch(parts[0], p) for p in patterns):
            continue
        if changed is not None and parts[1] not in changed:
            continue
        cands.append((parts[0], parts[1], int(parts[2])))
    if not cands:
        print(f"{categories}: no candidates")
        sys.exit(3)
    blocks = read(HERE / "triage-rules.md").split("\n## ")
    common, specific = blocks[0].split("\n"), []
    cats = {c[0] for c in cands}
    for block in blocks[1:]:
        if cats & set(re.split(r"[\s·,]+", block.split("\n", 1)[0])):
            specific += ("## " + block).rstrip().split("\n") + [""]
    name = re.sub(r"[^a-z0-9]+", "-", categories.lower()).strip("-")
    rows = outline(work)
    chunks = [cands[i:i + limit] for i in range(0, len(cands), limit)]
    for k, chunk in enumerate(chunks, 1):
        lines = [f"# Paquete de triaje · {categories} · {commit(work)} · parte {k} de {len(chunks)}", "",
                 f"- SNAPSHOT: {work / 'src'}", f"- Candidatos: {len(chunk)}", ""]
        lines += common + ["", "## Reglas de estas categorías", ""] + specific + ["## Candidatos", ""]
        for n, (cat, rel, ln) in enumerate(chunk, 1):
            src = read(work / "src" / rel).split("\n")
            a, b = enclosing(rows, rel, ln, context)
            lines += [f"#### [{n}] {cat} · {rel}:{ln}"] + excerpt(src, a, b, rel)[1:]
        emit(work, f"triage-{name}-{k}", lines)


def main():
    args = sys.argv[1:]
    if len(args) < 3:
        sys.exit(__doc__)
    mode, work, target = args[0], Path(args[1]), args[2]
    incremental = "--incremental" in args

    def opt(flag, default):
        return int(args[args.index(flag) + 1]) if flag in args else default
    if mode == "review":
        topic = Path(target) if target.endswith(".md") else HERE / "topics" / f"{target}.md"
        review(work, topic, incremental)
    elif mode == "triage":
        triage(work, target, opt("--limit", 40), opt("--context", 6), incremental)
    else:
        sys.exit(__doc__)


main()
