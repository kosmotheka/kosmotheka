#!/usr/bin/env python3
"""Mechanical facts for the Kosmotheka audit.

Runs inside the pinned Python image started by prepare.sh. Reads the snapshot
and the scanner outputs and writes plain-text facts. No network, no LLM.

Usage: facts.py <snapshot> <tools> <out>
"""
import json
import re
import sqlite3
import sys
import tomllib
import traceback
from collections import Counter, defaultdict
from pathlib import Path

SNAP, TOOLS, OUT = (Path(a) for a in sys.argv[1:4])
SKIP = ("docs/historico/", "docs/research/", "docs/design/", "app/src-tauri/gen/",
        "app/src/components/ui/", "brand/", "ds-bundle/")
INJECTED = ("State<", "AppHandle", "Window", "Webview", "Channel<")
LIMIT = 80
summary = []


def rel(p):
    return p.relative_to(SNAP).as_posix()


def text(p):
    path = SNAP / p if isinstance(p, str) else p
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


def sources(root, exts):
    base = SNAP / root
    if not base.exists():
        return []
    return [p for p in sorted(base.rglob("*"))
            if p.is_file() and p.suffix in exts and not rel(p).startswith(SKIP)]


def short(s, n=150):
    s = " ".join(str(s).split())
    return s if len(s) <= n else s[: n - 1] + "…"


def line_at(src, offset):
    return src.count("\n", 0, offset) + 1


def write(name, lines):
    (OUT / name).write_text("\n".join(lines) + "\n", encoding="utf-8")


def load_json(path):
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except Exception:
        return None


# ---------------------------------------------------------------- masking

RAW = re.compile(r'b?r(#*)"')
CHAR = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'")


def mask(src, lang):
    """Blank comments and string literals, keeping offsets. Returns (masked, literals)."""
    out, lits = list(src), []
    i, n = 0, len(src)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        if src.startswith("//", i) or (lang == "php" and c == "#" and not src.startswith("#[", i)):
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
            continue
        if src.startswith("/*", i):
            j = src.find("*/", i + 2)
            j = n if j < 0 else j + 2
            blank(i, j)
            i = j
            continue
        if lang == "rust":
            m = RAW.match(src, i)
            if m and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
                close = '"' + m.group(1)
                j = src.find(close, m.end())
                j = n if j < 0 else j + len(close)
                lits.append((i, j, src[m.end():max(m.end(), j - len(close))]))
                blank(i, j)
                i = j
                continue
            if c == "'":
                m = CHAR.match(src, i)
                if m:
                    blank(i, m.end())
                    i = m.end()
                else:
                    i += 1
                continue
        quotes = '"' if lang == "rust" else "\"'`"
        if c in quotes:
            multiline = lang == "rust" or c == "`"
            j = i + 1
            while j < n and src[j] != c and (multiline or src[j] != "\n"):
                j += 2 if src[j] == "\\" else 1
            j = min(j + 1, n)
            lits.append((i, j, src[i + 1:j - 1]))
            blank(i, j)
            i = j
            continue
        i += 1
    return "".join(out), lits


def block_end(masked, start):
    """End offset of the block opened by the first '{' after start; None if ';' comes first."""
    i, n = start, len(masked)
    while i < n and masked[i] not in "{;":
        i += 1
    if i >= n or masked[i] == ";":
        return None
    depth = 0
    while i < n:
        if masked[i] == "{":
            depth += 1
        elif masked[i] == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


# ---------------------------------------------------------------- parsing

RUST_FN = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:const[ \t]+)?(async[ \t]+)?(?:unsafe[ \t]+)?fn[ \t]+(\w+)", re.M)
PHP_FN = re.compile(r"^[ \t]*(?:(?:public|protected|private|static|final|abstract)[ \t]+)*function[ \t]+(\w+)", re.M)
TS_FN = re.compile(r"^(?:export[ \t]+)?(?:default[ \t]+)?(?:async[ \t]+)?function[ \t]+(\w+)|^(?:export[ \t]+)?const[ \t]+(\w+)[ \t]*(?::[^=\n]+)?=[ \t]*(?:async[ \t]*)?\(", re.M)

FILES = {}      # path -> dict(src, masked, lits, lang, tests=[(a, b)])
OUTLINE = []    # dicts: path, lang, name, start, end, test, async, command, sig


def lang_of(p):
    return {".rs": "rust", ".php": "php", ".ts": "ts", ".tsx": "ts"}.get(p.suffix)


def load_sources():
    roots = [("crates", {".rs"}), ("app/src-tauri/src", {".rs"}), ("app/src-tauri/tests", {".rs"}),
             ("app/src", {".ts", ".tsx"}), ("app/scripts", {".mjs", ".ts"}), ("server/src", {".php"}),
             ("server/tests", {".php"})]
    for root, exts in roots:
        for p in sources(root, exts):
            lang = lang_of(p) or "ts"
            src = text(p)
            masked, lits = mask(src, lang)
            info = {"src": src, "masked": masked, "lits": lits, "lang": lang, "tests": []}
            r = rel(p)
            if lang == "rust":
                if "/tests/" in "/" + r:
                    info["tests"].append((0, len(src)))
                for m in re.finditer(r"#\[cfg\(test\)\]\s*(?:pub\s+)?mod\s+\w+\s*", masked):
                    end = block_end(masked, m.end())
                    if end:
                        info["tests"].append((m.start(), end))
            FILES[r] = info
            outline_file(r, info)


def in_tests(info, offset):
    return any(a <= offset < b for a, b in info["tests"])


def outline_file(path, info):
    src, masked, lang = info["src"], info["masked"], info["lang"]
    if lang == "rust":
        for m in RUST_FN.finditer(masked):
            end = block_end(masked, m.end())
            if end is None:
                continue
            before = src[max(0, m.start() - 300):m.start()]
            attrs = before.split("\n}")[-1]
            command = "#[tauri::command" in attrs
            rename = re.search(r'rename_all\s*=\s*"(\w+)"', attrs) if command else None
            test = in_tests(info, m.start()) or "#[test]" in attrs or "#[tokio::test]" in attrs
            OUTLINE.append({"path": path, "lang": lang, "name": m.group(2), "start": line_at(src, m.start()),
                            "end": line_at(src, end - 1), "test": test, "async": bool(m.group(1)),
                            "command": command, "rename": rename.group(1) if rename else "",
                            "sig": src[m.start():masked.find("{", m.end())], "body": (m.start(), end)})
    elif lang == "php":
        for m in PHP_FN.finditer(masked):
            end = block_end(masked, m.end())
            if end is None:
                continue
            OUTLINE.append({"path": path, "lang": lang, "name": m.group(1), "start": line_at(src, m.start()),
                            "end": line_at(src, end - 1), "test": path.startswith("server/tests/"),
                            "async": False, "command": False, "rename": "", "sig": "", "body": (m.start(), end)})
    else:
        starts = [(line_at(src, m.start()), m.group(1) or m.group(2)) for m in TS_FN.finditer(masked)]
        total = src.count("\n") + 1
        for k, (start, name) in enumerate(starts):
            end = starts[k + 1][0] - 1 if k + 1 < len(starts) else total
            OUTLINE.append({"path": path, "lang": lang, "name": name, "start": start, "end": end,
                            "test": ".test." in path or ".spec." in path, "async": False,
                            "command": False, "rename": "", "sig": "", "body": None})


def section_outline():
    lines = ["# path\tstart\tend\tlang\tname\tflags"]
    for o in OUTLINE:
        flags = ",".join(f for f in ("test", "async", "command") if o[f])
        lines.append(f'{o["path"]}\t{o["start"]}\t{o["end"]}\t{o["lang"]}\t{o["name"]}\t{flags}')
    write("outline.tsv", lines)
    summary.append(f"- Outline: {len(OUTLINE)} functions in {len(FILES)} files (`outline.tsv`).")


# ---------------------------------------------------------------- candidates

CANDIDATES = []


def add(cat, path, line, code):
    CANDIDATES.append((cat, path, line, short(code)))


RUST_LINE_RULES = [
    ("rust.panic", re.compile(r"\.unwrap\(\)|\.expect\(|\bpanic!|\bunreachable!|\btodo!|\bunimplemented!")),
    ("rust.swallow", re.compile(r"\blet _ =|\.ok\(\);|unwrap_or_default\(\)|Err\(_\)\s*=>|map_err\(\|_\|")),
    ("rust.cast", re.compile(r"\bas (?:u8|u16|u32|i8|i16|i32)\b")),
    ("rust.allow", re.compile(r"#\[allow\((?:dead_code|unused)")),
    ("rust.unsafe", re.compile(r"\bunsafe\b")),
]
SQL_WRITE = re.compile(r"(?is)\b(?:insert\s+(?:or\s+\w+\s+)?into|replace\s+into|delete\s+from)\s+([a-z_]+)|\bupdate\s+([a-z_]+)\s+set\b")
SQL_WORD = re.compile(r"(?i)\b(select|insert|update|delete|where|from|order by|join)\b")
ACCENT = re.compile(r"[áéíóúñ¿¡ÁÉÍÓÚÑ]")
MARKER = re.compile(r"\b(TODO|FIXME|XXX|HACK)\b")


def catalog_tables():
    src = text("crates/domain/src/schema.rs")
    m = re.search(r"fn table\(self\)[^{]*\{(.*?)\n    \}", src, re.S)
    return set(re.findall(r'=>\s*"(\w+)"', m.group(1))) if m else set()


def section_candidates():
    tables = catalog_tables()
    for path, info in FILES.items():
        src, masked, lang = info["src"], info["masked"], info["lang"]
        raw_lines, code_lines = src.split("\n"), masked.split("\n")
        offsets, acc = [], 0
        for ln in raw_lines:
            offsets.append(acc)
            acc += len(ln) + 1
        for k, code in enumerate(code_lines, 1):
            test = in_tests(info, offsets[k - 1]) if lang == "rust" else ".test." in path
            raw = raw_lines[k - 1]
            if MARKER.search(raw) and raw.strip().startswith(("//", "#", "*", "/*")):
                add(f"{lang}.todo", path, k, raw)
            if test:
                continue
            if ACCENT.search(raw) and not path.startswith("app/src/i18n/"):
                add(f"{lang}.non-english", path, k, raw)
            if lang == "rust":
                for cat, rx in RUST_LINE_RULES:
                    if rx.search(code):
                        add(cat, path, k, raw)
            elif lang == "ts":
                ts_line(path, k, raw, code)
            elif lang == "php":
                if re.search(r"::from\(", code):
                    add("php.enum-from", path, k, raw)
                if re.search(r"new \\?(?:Exception|RuntimeException|LogicException)\(", code):
                    add("php.generic-exception", path, k, raw)
        if lang == "rust":
            rust_file_checks(path, info, tables)
        if lang == "php":
            if "declare(strict_types=1)" not in src:
                add("php.no-strict-types", path, 1, "declare(strict_types=1) missing")
            for m in re.finditer(r"catch\s*\([^)]*\)\s*\{\s*\}", masked):
                add("php.empty-catch", path, line_at(src, m.start()), src[m.start():m.end()])
    for o in OUTLINE:
        size = o["end"] - o["start"] + 1
        if not o["test"] and o["lang"] == "rust" and size > 80:
            add("rust.long-fn", o["path"], o["start"], f'fn {o["name"]}: {size} lines')
        if not o["test"] and o["lang"] == "rust" and o["async"]:
            a, b = o["body"]
            body = FILES[o["path"]]["masked"][a:b]
            if ".lock()" in body and ".await" in body:
                add("rust.lock-await", o["path"], o["start"], f'async fn {o["name"]} holds .lock() and .await')
    for path, info in FILES.items():
        n = info["src"].count("\n") + 1
        if (info["lang"] == "rust" and n > 800) or (info["lang"] == "ts" and n > 300):
            add(f'{info["lang"]}.big-file', path, 1, f"{n} lines")
    for r in (load_json(TOOLS / "semgrep.json") or {}).get("results", []):
        p = r["path"][5:] if r["path"].startswith("/src/") else r["path"]
        if not p.startswith(SKIP):
            add("semgrep", p, r["start"]["line"], f'{r["check_id"]}: {r["extra"].get("message", "")}')
    CANDIDATES.sort(key=lambda c: (c[0], c[1], c[2]))
    lines = ["# category\tpath\tline\tcode"] + [f"{c}\t{p}\t{l}\t{t}" for c, p, l, t in CANDIDATES]
    write("candidates.tsv", lines)
    counts = Counter(c[0] for c in CANDIDATES)
    summary.append("- Candidates for triage (`candidates.tsv`): " +
                   ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))


def rust_file_checks(path, info, tables):
    src, masked = info["src"], info["masked"]
    allowed = path.endswith(("src/ops.rs", "src/sync.rs"))
    for a, b, content in info["lits"]:
        if in_tests(info, a):
            continue
        for m in SQL_WRITE.finditer(content):
            table = m.group(1) or m.group(2)
            kind = "catalog" if table in tables else "other"
            if kind == "catalog" and not allowed:
                add("rust.sql-write-catalog", path, line_at(src, a), f"{table}: {content}")
            elif kind == "other" or not allowed:
                add("rust.sql-write-other", path, line_at(src, a), f"{table}: {content}")
    for m in re.finditer(r"format!\s*\(", masked):
        if in_tests(info, m.start()):
            continue
        nxt = next((l for l in info["lits"] if l[0] >= m.end()), None)
        if nxt and nxt[0] - m.end() < 20 and SQL_WORD.search(nxt[2]) and "{" in nxt[2]:
            add("rust.sql-format", path, line_at(src, m.start()), nxt[2])


def ts_line(path, k, raw, code):
    in_api = path.endswith("lib/api.ts") or "/dev/" in path
    if not in_api and (re.search(r"\binvoke\s*(<|\()", code) or "@tauri-apps/api/core" in raw):
        add("ts.ipc-outside-api", path, k, raw)
    if path.endswith(".tsx"):
        if re.search(r">\s*[^<>{}\n]*[A-Za-zÁÉÍÓÚáéíóúñÑ]{3,}[^<>{}\n]*</", raw) and "{t(" not in raw:
            add("ts.hardcoded-text", path, k, raw)
        if re.search(r'\b(placeholder|title|aria-label|alt|label|description)="[^"]*[A-Za-z]{3,}', raw):
            add("ts.hardcoded-attr", path, k, raw)
    rules = [("ts.any", r":\s*any\b|\bas any\b|as unknown as"), ("ts.index-key", r"key=\{(?:i|idx|index)\}"),
             ("ts.console", r"console\.(?:log|debug|info)\("), ("ts.listen", r"\blisten\s*(?:<[^>]*>)?\s*\("),
             ("ts.sink", r"dangerouslySetInnerHTML|\.innerHTML|window\.open\(|\beval\(|new Function\(|href=\{|src=\{"),
             ("ts.role-check", r"\brole\b\s*[!=]==|[!=]==\s*\w*\.?role\b|\b(?:isAdmin|canEdit|canWrite|canShare)\b"),
             ("ts.effect", r"\buseEffect\s*\(")]
    for cat, rx in rules:
        if re.search(rx, code if cat != "ts.sink" else raw):
            add(cat, path, k, raw)
    if re.search(r"@ts-ignore|@ts-expect-error", raw):
        add("ts.any", path, k, raw)
    if "mock-ipc" in raw and "import" in raw:
        add("ts.mock-ipc-import", path, k, raw)


# ---------------------------------------------------------------- contracts

def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def camel(name):
    head, *rest = name.split("_")
    return head + "".join(w.title() for w in rest)


def rust_schema():
    src = text("crates/domain/src/schema.rs")
    domain = "\n".join(text(p) for p in sources("crates/domain/src", {".rs"}))
    lists = {n: re.findall(r'"(\w+)"', body) for n, body in re.findall(r"const (\w+): &\[&str\] = &\[(.*?)\];", domain, re.S)}
    consts = {}
    for m in re.finditer(r"const (\w+)_FIELDS: &\[FieldSpec\] = &\[(.*?)\];", src, re.S):
        fields = {}
        for f in re.finditer(r'field\(\s*"(\w+)",\s*FieldType::(\w+)(?:\((.*?)\))?,\s*(true|false)\s*\)', m.group(2), re.S):
            arg = (f.group(3) or "").strip().lstrip("&")
            values = (lists.get(arg) or re.findall(r'"(\w+)"', arg)) if f.group(2) == "OneOf" else []
            fields[f.group(1)] = {"type": f.group(2).lower(), "required": f.group(4) == "true", "values": values}
        consts[m.group(1)] = fields
    mapping = {}
    for m in re.finditer(r"((?:Entity::\w+\s*\|\s*)*Entity::\w+)\s*=>\s*(\w+)_FIELDS", src):
        for e in re.findall(r"Entity::(\w+)", m.group(1)):
            mapping[snake(e)] = consts.get(m.group(2), {})
    for name, fields in consts.items():
        mapping.setdefault(name.lower(), fields)
    t = re.search(r"fn table\(self\)[^{]*\{(.*?)\n    \}", src, re.S)
    tables = {snake(e): tb for e, tb in re.findall(r'Entity::(\w+)\s*=>\s*"(\w+)"', t.group(1))} if t else {}
    return mapping, tables


def sql_schema():
    db = sqlite3.connect(":memory:")
    errors = []
    for p in sorted((SNAP / "app/src-tauri/migrations").glob("*.sql")):
        try:
            db.executescript(p.read_text(encoding="utf-8"))
        except Exception as e:
            errors.append(f"{p.name}: {e}")
    tables = {}
    for (name, sql) in db.execute("select name, sql from sqlite_master where type='table'"):
        cols = {}
        for _, col, typ, notnull, default, pk in db.execute(f"pragma table_info('{name}')"):
            check = re.search(rf"\b{col}\b[^,]*?CHECK\s*\(\s*{col}\s+IN\s*\(([^)]*)\)", sql or "", re.I)
            cols[col] = {"type": typ, "notnull": bool(notnull or pk),
                         "values": re.findall(r"'(\w+)'", check.group(1)) if check else []}
        fks = [f"{r[3]}->{r[2]}.{r[4]}" for r in db.execute(f"pragma foreign_key_list('{name}')")]
        tables[name] = {"cols": cols, "fks": fks}
    return tables, errors


def json_schema():
    d = load_json(SNAP / "spec/operation.schema.json") or {}
    out = {}
    for item in d.get("$defs", {}).get("entityFields", {}).get("allOf", []):
        ent = item.get("if", {}).get("properties", {}).get("entity", {})
        names = [ent["const"]] if "const" in ent else ent.get("enum", [])
        fields_schema = item.get("then", {}).get("properties", {}).get("fields", {})
        props = fields_schema.get("properties", {})
        for n in names:
            fields = out.setdefault(n, {})
            for f, s in props.items():
                if "enum" in s:
                    fields[f] = {"type": "oneof", "values": [v for v in s["enum"] if v is not None]}
                elif "$ref" in s:
                    fields[f] = {"type": s["$ref"].rsplit("/", 1)[-1].lower(), "values": []}
                else:
                    fields[f] = {"type": str(s.get("type", "?")).lower(), "values": []}
    return out


def php_schema():
    src = text("server/src/Enum/CatalogEntity.php")
    cases = dict(re.findall(r"case (\w+) = '(\w+)';", src))
    lists = {n: re.findall(r"'(\w+)'", body) for n, body in re.findall(r"const (?:\w+\s+)?(\w+) = \[(.*?)\];", src, re.S)}
    m = re.search(r"function fields\(\): array\s*\{(.*?)\n    \}", src, re.S)
    body = m.group(1) if m else ""
    out = {}
    for arm in re.finditer(r"((?:self::\w+\s*,\s*)*self::\w+)\s*=>\s*\[", body):
        depth, i = 1, arm.end()
        while i < len(body) and depth:
            depth += {"[": 1, "]": -1}.get(body[i], 0)
            i += 1
        entities = [cases.get(c, snake(c)) for c in re.findall(r"self::(\w+)", arm.group(1))]
        items, cur, level = [], "", 0
        for ch in body[arm.end():i - 1]:
            level += {"(": 1, "[": 1, ")": -1, "]": -1}.get(ch, 0)
            if ch == "," and level == 0:
                items.append(cur)
                cur = ""
            else:
                cur += ch
        items.append(cur)
        fields = {}
        for item in items:
            f = re.match(r"\s*'(\w+)'\s*=>\s*(.+)", item, re.S)
            if not f:
                continue
            v = " ".join(f.group(2).split())
            named = [x for n, x in lists.items() if re.search(rf"self::{n}\b", v)]
            h = re.match(r"\$(\w+)\((.*)\)$", v)
            if h:
                kind = h.group(1).lower()
                values = (named[0] if named else re.findall(r"'(\w+)'", h.group(2))) if kind == "oneof" else []
                fields[f.group(1)] = {"type": kind, "required": kind == "oneof" or "true" in h.group(2), "values": values}
            else:
                n = re.search(r"FieldType::(\w+),\s*(true|false)", v)
                fields[f.group(1)] = {"type": n.group(1).lower() if n else "?", "required": bool(n and n.group(2) == "true"),
                                      "values": named[0] if named else re.findall(r"'(\w+)'", v)}
        for e in entities:
            out[e] = fields
    return out, cases


def section_schema(lines):
    rust, tables = rust_schema()
    sql, errors = sql_schema()
    js, php = json_schema(), php_schema()[0]
    diffs = 0
    lines.append("## Schema chain: schema.rs · migrations · operation.schema.json · CatalogEntity.php")
    for e in errors:
        lines.append(f"- ⚠ migration failed to apply: {e}")
    entities = sorted(set(rust) | set(js) | set(php))
    ok = []
    for e in entities:
        r, j, p = rust.get(e, {}), js.get(e, {}), php.get(e, {})
        table = tables.get(e)
        cols = sql.get(table, {}).get("cols", {}) if table else {}
        issues = []
        if not r:
            issues.append("missing in schema.rs")
        if not table or table not in sql:
            issues.append(f"no table for entity (schema.rs says {table!r})")
        for f in sorted(set(r) | set(j) | set(p)):
            where = [n for n, d in (("schema.rs", r), ("json", j), ("php", p)) if f not in d]
            if where:
                issues.append(f"{f}: missing in {', '.join(where)}")
            if f in r and cols and f not in cols:
                issues.append(f"{f}: no column in table {table}")
            rv = sorted(map(str, r.get(f, {}).get("values", [])))
            for name, d in (("json", j), ("php", p)):
                v = sorted(map(str, d.get(f, {}).get("values", [])))
                if f in r and f in d and (rv or v) and rv != v:
                    issues.append(f"{f}: values schema.rs {rv} vs {name} {v}")
            if f in r and f in cols and cols[f]["values"] and sorted(map(str, cols[f]["values"])) != rv:
                issues.append(f"{f}: values schema.rs {rv} vs SQL CHECK {sorted(cols[f]['values'])}")
            if f in r and f in p and r[f]["required"] != p[f]["required"]:
                issues.append(f"{f}: required schema.rs {r[f]['required']} vs php {p[f]['required']}")
            if f in r and f in p and r[f]["type"] != p[f]["type"]:
                issues.append(f"{f}: type schema.rs {r[f]['type']} vs php {p[f]['type']}")
            if f in r and f in cols and r[f]["required"] and not cols[f]["notnull"]:
                issues.append(f"{f}: required in schema.rs but nullable column")
        if issues:
            diffs += 1
            lines.append(f"- **{e}** (table `{table}`): " + "; ".join(issues))
        else:
            ok.append(e)
    lines.append(f"- Consistent: {', '.join(ok) or '—'}")
    lines.append("")
    lines.append("### Tables built from the migrations (columns; FKs)")
    for t, d in sorted(sql.items()):
        cols = ", ".join(f"{c}{'!' if v['notnull'] else ''}" for c, v in d["cols"].items())
        lines.append(f"- `{t}`: {cols}" + (f" · FK {', '.join(d['fks'])}" if d["fks"] else ""))
    lines.append("")
    summary.append(f"- Schema chain: {diffs} entities with differences, {len(errors)} migrations failed to apply (`contracts.md`).")


def section_commands(lines):
    lib = text("app/src-tauri/src/lib.rs")
    m = re.search(r"generate_handler!\[(.*?)\]", lib, re.S)
    registered = [x.strip().split("::")[-1] for x in m.group(1).split(",") if x.strip()] if m else []
    defs = {o["name"]: o for o in OUTLINE if o["command"]}
    invokes = []
    for path, info in FILES.items():
        if info["lang"] != "ts":
            continue
        for mm in re.finditer(r"\binvoke\s*(?:<[^()]*?>)?\(\s*[\"'`](\w+)[\"'`]\s*(?:,\s*(\{[^{}]*\}|\w+))?", info["src"]):
            keys = None
            if mm.group(2) and mm.group(2).startswith("{"):
                keys = [k.strip().split(":")[0].strip() for k in mm.group(2)[1:-1].split(",") if k.strip()]
                keys = [k for k in keys if k and not k.startswith("...")]
            invokes.append((mm.group(1), keys, path, line_at(info["src"], mm.start())))
    invoked = {i[0] for i in invokes}
    lines.append("## Tauri commands and the UI")
    missing = sorted(invoked - set(registered))
    unused = sorted(set(registered) - invoked)
    lines.append(f"- Registered: {len(registered)} · invoked from the UI: {len(invoked)}")
    lines.append(f"- ⚠ Invoked but not registered: {', '.join(missing) or '—'}")
    lines.append(f"- Registered but never invoked: {', '.join(unused) or '—'}")
    lines.append(f"- Registered without a `#[tauri::command]` definition found: {', '.join(sorted(set(registered) - set(defs))) or '—'}")
    mismatches = 0
    for name, keys, path, ln in invokes:
        d = defs.get(name)
        if not d or keys is None:
            continue
        params = []
        sig = d["sig"]
        start = sig.find("(")
        depth, cur, items = 0, "", []
        for ch in sig[start + 1:]:
            if ch in "<([":
                depth += 1
            elif ch in ">)]":
                if depth == 0:
                    items.append(cur)
                    break
                depth -= 1
            if ch == "," and depth == 0:
                items.append(cur)
                cur = ""
                continue
            cur += ch
        for it in items:
            if ":" not in it:
                continue
            pname, ptype = it.split(":", 1)
            pname = pname.strip().removeprefix("mut ").strip()
            if pname == "self" or any(x in ptype for x in INJECTED):
                continue
            opt = "Option<" in ptype
            params.append((pname if d["rename"] == "snake_case" else camel(pname), opt))
        expected = {p for p, _ in params}
        required = {p for p, opt in params if not opt}
        extra, lacking = set(keys) - expected, required - set(keys)
        if extra or lacking:
            mismatches += 1
            lines.append(f"- ⚠ `{name}` at {path}:{ln}: sends {sorted(keys)}, command expects {sorted(expected)}"
                         f" (required {sorted(required)}) — defined at {d['path']}:{d['start']}")
    emits, listens, consts = set(), set(), set()
    for path, info in FILES.items():
        if info["lang"] == "rust":
            emits.update(re.findall(r"\.emit(?:_to)?\(\s*(?:[^,()\"]+,\s*)?\"([\w:.-]+)\"", info["src"]))
            consts.update(re.findall(r"const \w*EVENT\w*: &str = \"([\w:.-]+)\"", info["src"]))
        elif info["lang"] == "ts":
            listens.update(re.findall(r"\blisten\s*(?:<[^>]*>)?\(\s*[\"'`]([\w:.-]+)", info["src"]))
    lines.append(f"- Events emitted by Rust: {', '.join(sorted(emits)) or '—'} (string constants that may be event names: {', '.join(sorted(consts - emits)) or '—'})")
    lines.append(f"- Events listened to by the UI: {', '.join(sorted(listens)) or '—'}")
    lost = sorted(listens - emits - consts)
    lines.append(f"- ⚠ Listened to but never emitted: {', '.join(lost) or '—'}")
    lines.append("")
    summary.append(f"- Commands: {len(missing)} invoked but not registered, {len(unused)} never invoked, {mismatches} argument mismatches; events listened but not emitted: {len(lost)}.")


def flatten(d, prefix=""):
    for k, v in d.items():
        key = f"{prefix}.{k}" if prefix else k
        if isinstance(v, dict):
            yield from flatten(v, key)
        else:
            yield key, v


def section_i18n(lines):
    base = SNAP / "app/src/i18n/locales"
    en = dict(flatten(load_json(base / "en.json") or {}))
    es = dict(flatten(load_json(base / "es.json") or {}))
    variables = lambda s: sorted(set(re.findall(r"{{\s*([\w.]+)\s*}}", str(s))))
    used, dynamic, counted = defaultdict(list), [], set()
    for path, info in FILES.items():
        if info["lang"] != "ts":
            continue
        src = info["src"]
        for m in re.finditer(r"\bt\(\s*[\"'`]([\w.:-]+)[\"'`](\s*,\s*\{[^}]*\bcount\b)?", src):
            used[m.group(1)].append(f"{path}:{line_at(src, m.start())}")
            if m.group(2):
                counted.add(m.group(1))
        for m in re.finditer(r"i18nKey=[\"']([\w.:-]+)", src):
            used[m.group(1)].append(f"{path}:{line_at(src, m.start())}")
        for m in re.finditer(r"\bt\(\s*`([^`]*?)\$\{", src):
            dynamic.append((m.group(1), f"{path}:{line_at(src, m.start())}"))
    plural = lambda k, d: k in d or any(x in d for x in (k + "_one", k + "_other"))
    missing = sorted(k for k in used if not (plural(k, en) and plural(k, es)))
    no_plural = sorted(k for k in counted if not all(k + s in d for s in ("_one", "_other") for d in (en, es)))
    prefixes = [p for p, _ in dynamic if p]
    unused = sorted(k for k in en if k not in used and re.sub(r"_(one|other|many|zero)$", "", k) not in used
                    and not any(k.startswith(p) for p in prefixes))
    lines.append("## Translations")
    lines.append(f"- Keys: en {len(en)}, es {len(es)}; used in code {len(used)}; dynamic key sites {len(dynamic)}")
    for label, items in (("⚠ Only in en", sorted(set(en) - set(es))), ("⚠ Only in es", sorted(set(es) - set(en))),
                         ("⚠ Used in code but missing", [f"{k} ({used[k][0]})" for k in missing]),
                         ("⚠ Used with count but without _one/_other in both", no_plural),
                         ("⚠ Different {{variables}} in en and es", [k for k in en if k in es and variables(en[k]) != variables(es[k])]),
                         ("Empty values", [k for d in (en, es) for k, v in d.items() if not str(v).strip()]),
                         ("Same text in en and es", [k for k in en if k in es and en[k] == es[k] and re.search(r"[a-z]{4}", str(en[k]))]),
                         ("Dynamic key sites (prefix · where)", [f"`{p}…` {w}" for p, w in dynamic]),
                         ("Defined but never used (static check)", unused)):
        lines.append(f"- {label} ({len(items)}): {', '.join(items[:LIMIT]) or '—'}" + (" …" if len(items) > LIMIT else ""))
    lines.append("")
    summary.append(f"- Translations: only-en {len(set(en) - set(es))}, only-es {len(set(es) - set(en))}, used-but-missing {len(missing)}, never used {len(unused)}.")


def section_errors(lines):
    code = re.compile(r"^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)+$")
    found = defaultdict(set)
    for f in sorted((SNAP / "server/src/Enum").glob("*ErrorCode.php")):
        for v in re.findall(r"case \w+ = '([\w.-]+)';", text(f)):
            found[v].add("php")
    for path, info in FILES.items():
        if path.endswith("lib/sync-errors.ts"):
            for _, _, s in info["lits"]:
                if code.match(s):
                    found[s].add("ts")
        if info["lang"] == "rust" and not path.endswith("schema.rs"):
            for a, _, s in info["lits"]:
                if code.match(s) and not in_tests(info, a) and ("error" in path or "code" in info["src"][max(0, a - 80):a].lower()):
                    found[s].add("rust")

    def walk(node):
        if isinstance(node, dict):
            for k, v in node.items():
                if k in ("error", "code", "rejected", "reason") and isinstance(v, str) and code.match(v):
                    found[v].add("spec")
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)
    for f in sorted((SNAP / "spec/conformance").glob("*.json")):
        walk(load_json(f))
    en = dict(flatten(load_json(SNAP / "app/src/i18n/locales/en.json") or {}))
    for k in en:
        if k.startswith("errors.") or ".errors." in k:
            found[k.split(".")[-1]].add("i18n")
    for tok in re.findall(r"`([a-z][a-z0-9]*(?:[._-][a-z0-9]+)+)`", text("docs/design/errors.md")):
        found[tok].add("docs")
    lines.append("## Error codes by source (heuristic: rust, php, spec, ts, i18n, docs)")
    for c in sorted(found):
        lines.append(f"- `{c}`: {', '.join(sorted(found[c]))}")
    lines.append("")
    summary.append(f"- Error codes: {len(found)} distinct codes collected across sources (`contracts.md`).")


def section_conformance(lines):
    lines.append("## Conformance cases and their runners")
    hay = {path: info["src"] for path, info in FILES.items()}
    for extra in ("app/scripts/check-spec.mjs",):
        hay[extra] = text(extra)
    for f in sorted((SNAP / "spec/conformance").glob("*.json")):
        data = load_json(f)
        cases = data if isinstance(data, list) else (data or {}).get("cases", [])
        runners = sorted({p for p, s in hay.items() if f.name in s or f.stem in s})
        sides = {"rust" if p.endswith(".rs") else "php" if p.endswith(".php") else "js" for p in runners}
        flag = "" if {"rust", "php"} <= sides else " ⚠ not run by both Rust and PHP"
        lines.append(f"- `{f.name}`: {len(cases)} cases · runners: {', '.join(runners) or '—'}{flag}")
        for c in cases:
            if isinstance(c, dict):
                exp = {k: v for k, v in c.items() if re.search(r"expect|error|valid|reject|result|outcome", k, re.I)}
                lines.append(f"  - {short(c.get('name', '?'), 110)} → {short(json.dumps(exp, ensure_ascii=False), 140)}")
    lines.append("")


def section_contracts():
    lines = ["# Contracts between layers", ""]
    for fn in (section_schema, section_commands, section_i18n, section_errors, section_conformance):
        try:
            fn(lines)
        except Exception:
            lines.append(f"- ⚠ {fn.__name__} failed: {traceback.format_exc(limit=2)}")
            summary.append(f"- ⚠ {fn.__name__} failed (see `contracts.md`).")
    write("contracts.md", lines)


# ---------------------------------------------------------------- surface

def numbered(path, limit=220):
    src = text(path)
    rows = src.split("\n")
    if not src:
        return [f"(missing: {path})"]
    body = [f"{i:4} {l}" for i, l in enumerate(rows[:limit], 1)]
    return [f"### {path}", "```"] + body + (["… (truncated)"] if len(rows) > limit else []) + ["```"]


def env_value(v):
    v = v.strip().strip("\"'")
    if not v:
        return "empty"
    if re.search(r"change.?me|example|placeholder|secret$|your[_-]", v, re.I):
        return "placeholder"
    if "://" in v:
        return "url " + re.sub(r"://([^:/@]+):([^@]+)@", r"://\1:***@", v)
    if re.fullmatch(r"[0-9a-fA-F]{16,}|[A-Za-z0-9+/=_-]{24,}", v):
        return f"random-looking value ({len(v)} chars)"
    return v if len(v) <= 40 and not re.search(r"pass|secret|key|token", v, re.I) else f"value ({len(v)} chars)"


def section_surface():
    lines = ["# Attack surface and configuration", ""]
    conf = load_json(SNAP / "app/src-tauri/tauri.conf.json") or {}
    app = conf.get("app", {})
    lines += ["## tauri.conf.json", f"- app.security: `{json.dumps(app.get('security'))}`",
              f"- app.withGlobalTauri: {app.get('withGlobalTauri')}", f"- plugins: `{json.dumps(conf.get('plugins'))}`",
              f"- windows: `{short(json.dumps(app.get('windows')), 300)}`", ""]
    lines.append("## Capabilities")
    for f in sorted((SNAP / "app/src-tauri/capabilities").glob("*.json")):
        c = load_json(f) or {}
        perms = [p if isinstance(p, str) else f"{p.get('identifier')} {json.dumps({k: v for k, v in p.items() if k != 'identifier'})}"
                 for p in c.get("permissions", [])]
        lines.append(f"- `{f.name}`: windows {c.get('windows')}, platforms {c.get('platforms')}, remote {c.get('remote')}")
        lines += [f"  - {p}" for p in perms]
    manifest = text("app/src-tauri/gen/android/app/src/main/AndroidManifest.xml")
    lines += ["", "## AndroidManifest.xml"]
    for attr in ("allowBackup", "usesCleartextTraffic", "networkSecurityConfig", "dataExtractionRules",
                 "fullBackupContent", "debuggable", "exported"):
        vals = re.findall(rf'android:{attr}="([^"]*)"', manifest)
        lines.append(f"- android:{attr}: {', '.join(vals) if vals else 'not set'}")
    lines.append(f"- permissions: {', '.join(re.findall(r'uses-permission[^>]*android:name=\"([^\"]+)\"', manifest)) or '—'}")
    lines.append(f"- intent schemes: {', '.join(re.findall(r'android:scheme=\"([^\"]+)\"', manifest)) or '—'}")
    lines += ["", "## Environment files (values classified, never printed)"]
    for f in sorted(list((SNAP / "server").glob(".env*")) + list((SNAP / "server/docker").glob(".env*"))):
        for k, v in re.findall(r"^([A-Z0-9_]+)=(.*)$", text(f), re.M):
            lines.append(f"- {rel(f)}: {k} = {env_value(v)}")
    lines.append("")
    for path in ("server/docker/docker-compose.yaml", "server/docker/nginx/kosmotheka.local.conf",
                 "server/docker/nginx/nginx.conf", "server/docker/nginx/Dockerfile", "server/docker/php/Dockerfile",
                 "server/config/packages/security.yaml", "server/config/packages/framework.yaml", ".github/workflows/ci.yml"):
        lines += numbered(path) + [""]
    write("surface.md", lines)
    summary.append("- Surface: Tauri security config, capabilities, Android manifest, env keys, Docker, nginx, security.yaml and CI (`surface.md`).")


# ---------------------------------------------------------------- tests and docs

def section_tests():
    lines = ["# Tests", "", "## Rust tests per file"]
    per = defaultdict(list)
    for o in OUTLINE:
        if o["lang"] == "rust" and o["test"]:
            src = FILES[o["path"]]["src"]
            head = "\n".join(src.split("\n")[max(0, o["start"] - 4):o["start"]])
            if "#[test]" in head or "#[tokio::test]" in head:
                tag = " (ignored)" if "#[ignore" in head else ""
                per[o["path"]].append(o["name"] + tag)
    total = 0
    for path, names in sorted(per.items()):
        total += len(names)
        lines.append(f"- `{path}` ({len(names)}): {', '.join(names)}")
    ignored = [n for names in per.values() for n in names if n.endswith("(ignored)")]
    bad = [n for n in ignored if not n.startswith(("live_", "print_"))]
    lines.append(f"- Ignored: {', '.join(ignored) or '—'}; ⚠ ignored but not named live_* or print_*: {', '.join(bad) or '—'}")
    net = []
    for path, info in FILES.items():
        if info["lang"] != "rust":
            continue
        for a, b in info["tests"]:
            for m in re.finditer(r"reqwest|TcpStream|UdpSocket|https?://|\.bind\(", info["src"][a:b]):
                net.append(f"{path}:{line_at(info['src'], a + m.start())}")
    lines.append(f"- Network-looking code inside Rust tests: {', '.join(net[:LIMIT]) or '—'}")
    lines += ["", "## PHP tests"]
    for f in sorted((SNAP / "server/tests").rglob("*.php")):
        src = text(f)
        names = re.findall(r"function (test\w+)", src)
        lines.append(f"- `{rel(f)}`: {len(names)} tests, {len(re.findall(r'#\[(?:DataProvider|TestWith)', src))} data providers: {', '.join(names)}")
    ui = [rel(p) for p in (SNAP / "app").rglob("*") if re.search(r"\.(test|spec)\.[tj]sx?$", p.name) and "node_modules" not in p.parts]
    pkg = load_json(SNAP / "app/package.json") or {}
    lines += ["", f"## UI tests: {', '.join(ui) or 'none'}; package.json scripts: {', '.join(pkg.get('scripts', {}))}", "",
              "## CI steps"] + [f"- {s.strip()}" for s in re.findall(r"run: (.+)", text(".github/workflows/ci.yml"))]
    lines += ["", "## Manual checklists (docs/testing)"]
    for f in sorted((SNAP / "docs/testing").glob("*.md")):
        heads = re.findall(r"^#{2,3} (.+)$", text(f), re.M)
        lines.append(f"- `{f.name}`: {' · '.join(heads)}")
    write("tests.md", lines)
    summary.append(f"- Tests: {total} Rust tests in {len(per)} files, {len(ignored)} ignored, UI tests: {len(ui)} (`tests.md`).")


def section_docs():
    lines = ["# Docs index", "", "## Decisions (docs/DECISIONS.md)"]
    decisions = re.findall(r"^## (D\d+) · (.+?) — `([^`]+)`", text("docs/DECISIONS.md"), re.M)
    lines += [f"- {d} · {t} — {s}" for d, t, s in decisions]
    lines += ["", "## Roadmap status (docs/ROADMAP.md)"]
    for l in text("docs/ROADMAP.md").split("\n"):
        if l.startswith("#") or (l.startswith("|") and re.search(r"✅|⏳|🚧|\d", l)):
            lines.append(short(l, 200))
    lines += ["", "## Plans (docs/plans)"]
    for f in sorted((SNAP / "docs/plans").glob("*.md")):
        lines.append(f"- `{f.name}`: {' · '.join(re.findall(r'^## (.+)$', text(f), re.M))}")
    write("docs.md", lines)
    states = Counter(s for _, _, s in decisions)
    summary.append(f"- Decisions: {len(decisions)} ({', '.join(f'{k} {v}' for k, v in states.items())}) (`docs.md`).")


# ---------------------------------------------------------------- scanners

def deps():
    """Direct dependencies by kind and the runtime reachability of Rust crates."""
    kinds = {}
    for f in ("Cargo.toml", "crates/domain/Cargo.toml", "app/src-tauri/Cargo.toml"):
        try:
            t = tomllib.loads(text(f))
        except Exception:
            continue
        tables = [t] + list(t.get("target", {}).values())
        for tb in tables:
            for sec, kind in (("dependencies", "runtime"), ("dev-dependencies", "dev"), ("build-dependencies", "build")):
                for name in tb.get(sec, {}):
                    kinds.setdefault(("crates.io", name), kind)
    pkg = load_json(SNAP / "app/package.json") or {}
    for name in pkg.get("dependencies", {}):
        kinds[("npm", name)] = "runtime"
    for name in pkg.get("devDependencies", {}):
        kinds[("npm", name)] = "dev"
    comp = load_json(SNAP / "server/composer.json") or {}
    for name in comp.get("require", {}):
        kinds[("Packagist", name)] = "runtime"
    for name in comp.get("require-dev", {}):
        kinds[("Packagist", name)] = "dev"
    graph = defaultdict(set)
    try:
        for p in tomllib.loads(text("Cargo.lock")).get("package", []):
            for d in p.get("dependencies", []):
                graph[p["name"]].add(d.split()[0])
    except Exception:
        pass
    roots = [n for (eco, n), k in kinds.items() if eco == "crates.io" and k == "runtime"]
    seen, stack = set(roots), list(roots)
    while stack:
        for d in graph[stack.pop()]:
            if d not in seen:
                seen.add(d)
                stack.append(d)
    npm_dev = {k.split("node_modules/")[-1] for k, v in (load_json(SNAP / "app/package-lock.json") or {}).get("packages", {}).items() if v.get("dev")}
    lock = load_json(SNAP / "server/composer.lock") or {}
    php_dev = {p["name"] for p in lock.get("packages-dev", [])}
    parents = defaultdict(set)
    for p, ds in graph.items():
        for d in ds:
            parents[d].add(p)

    def classify(eco, name):
        direct = kinds.get((eco, name))
        if direct:
            return f"direct {direct}"
        if eco == "crates.io":
            ps = ", ".join(sorted(parents[name])[:3])
            return ("transitive runtime" if name in seen else "transitive dev/build only") + f" (via {ps})"
        if eco == "npm":
            return "transitive dev only" if name in npm_dev else "transitive runtime"
        return "transitive dev only" if name in php_dev else "transitive runtime"
    return classify


def section_scanners():
    lines = ["# Scanner results", ""]
    classify = deps()
    changed = set(text_tools("changed.txt").split()) if (TOOLS / "changed.txt").exists() else None
    osv = load_json(TOOLS / "osv.json") or {}
    vulns = []
    for res in osv.get("results", []):
        for pkg in res.get("packages", []):
            p = pkg.get("package", {})
            sev = {i: g.get("max_severity", "") for g in pkg.get("groups", []) for i in g.get("ids", [])}
            for v in pkg.get("vulnerabilities", []):
                fixed = sorted({e["fixed"] for a in v.get("affected", []) for r in a.get("ranges", [])
                                for e in r.get("events", []) if "fixed" in e})
                vulns.append(f"- {p.get('ecosystem')} `{p.get('name')}` {p.get('version')} · {v['id']} · severity {sev.get(v['id']) or '?'}"
                             f" · fixed in {', '.join(fixed) or 'none'} · {classify(p.get('ecosystem'), p.get('name'))} · {short(v.get('summary', ''), 110)}")
    lines += [f"## Vulnerable dependencies (osv-scanner): {len(vulns)}"] + vulns + [""]
    lic = load_json(TOOLS / "osv-licenses.json") or {}
    groups = defaultdict(list)
    allowed = {"MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "0BSD", "Zlib", "Unicode-3.0",
               "Unicode-DFS-2016", "CC0-1.0", "BSL-1.0", "Unlicense"}
    own = {"kosmotheka", "kosmotheka-domain", (load_json(SNAP / "app/package.json") or {}).get("name")}
    known = {}
    for k, v in (load_json(SNAP / "app/package-lock.json") or {}).get("packages", {}).items():
        if v.get("license"):
            known[("npm", k.split("node_modules/")[-1])] = str(v["license"])
    lockp = load_json(SNAP / "server/composer.lock") or {}
    for v in lockp.get("packages", []) + lockp.get("packages-dev", []):
        if v.get("license"):
            known[("Packagist", v["name"])] = " OR ".join(v["license"])
    for res in lic.get("results", []):
        for pkg in res.get("packages", []):
            p = pkg.get("package", {})
            if not pkg.get("license_violations") or p.get("name") in own:
                continue
            ls = " AND ".join(x for x in (pkg.get("licenses") or []) if x != "UNKNOWN") or known.get((p.get("ecosystem"), p.get("name")), "UNKNOWN")
            alts = [a.strip("() ") for a in re.split(r"\s+OR\s+", ls)]
            if any(all(x.strip("() ") in allowed for x in re.split(r"\s+AND\s+", a)) for a in alts):
                family = "permissive expression (an allowed option exists)"
            elif re.search(r"GPL|AGPL|EUPL|OSL|CC-BY-SA", ls):
                family = "copyleft"
            elif re.search(r"MPL|EPL|CDDL", ls):
                family = "weak copyleft (file level)"
            elif re.search(r"UNKNOWN|NOASSERTION|^$", ls):
                family = "unknown"
            else:
                family = "other"
            groups[(family, ls)].append(f"{p.get('name')} ({classify(p.get('ecosystem'), p.get('name')).split(' (')[0]})")
    lines.append("## Licenses outside the allowlist, grouped")
    for (family, ls), pkgs in sorted(groups.items()):
        if family.startswith("permissive"):
            continue
        lines.append(f"- [{family}] {ls}: {len(pkgs)} · {', '.join(pkgs[:6])}" + (" …" if len(pkgs) > 6 else ""))
    lines.append("")
    leaks = load_json(TOOLS / "gitleaks.json") or []
    lines.append(f"## Secrets in git history (gitleaks, redacted): {len(leaks)}")
    lines += [f"- {l.get('File')}:{l.get('StartLine')} · commit {str(l.get('Commit'))[:8]} {str(l.get('Date'))[:10]} · {l.get('RuleID')} · {short(l.get('Match', ''), 100)}" for l in leaks]
    lines.append("")
    sg = load_json(TOOLS / "semgrep.json") or {}
    by_rule = defaultdict(list)
    for r in sg.get("results", []):
        p = r["path"][5:] if r["path"].startswith("/src/") else r["path"]
        if p.startswith(SKIP) or (changed is not None and p not in changed):
            continue
        by_rule[(r["check_id"], r["extra"].get("severity", ""), short(r["extra"].get("message", ""), 140))].append(f"{p}:{r['start']['line']}")
    total = sum(len(v) for v in by_rule.values())
    lines.append(f"## Semgrep results{' in changed files' if changed is not None else ''}: {total} (errors: {len(sg.get('errors', []))})")
    for (rule, sev, msg), locs in sorted(by_rule.items()):
        lines.append(f"- `{rule}` [{sev}] {len(locs)}× · {msg} · {', '.join(locs[:10])}" + (" …" if len(locs) > 10 else ""))
    write("scanners.md", lines)
    fam = Counter(f for f, _ in groups)
    summary.append(f"- Scanners: {len(vulns)} vulnerable dependencies, licenses {dict(fam)}, {len(leaks)} gitleaks hits, {total} semgrep results (`scanners.md`).")


def text_tools(name):
    p = TOOLS / name
    return p.read_text(encoding="utf-8") if p.exists() else ""


# ---------------------------------------------------------------- changes

def section_changes():
    patch = text_tools("diff.patch")
    if not patch:
        summary.append("- Mode: initial (no diff).")
        return
    hunks, path = [], None
    for l in patch.split("\n"):
        if l.startswith("+++ "):
            path = None if l.endswith("/dev/null") else l[6:]
        m = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", l)
        if m and path:
            start, count = int(m.group(1)), int(m.group(2) or 1)
            hunks.append((path, start, start + max(count, 1) - 1))
    touched = {}
    for p, a, b in hunks:
        for o in OUTLINE:
            if o["path"] == p and o["start"] <= b and a <= o["end"]:
                touched[(p, o["name"], o["start"], o["end"])] = True
    lines = ["# Changes since the previous audit", "", "## Commits"] + [f"- {c}" for c in text_tools("commits.txt").split("\n") if c]
    lines += ["", "## Changed files"] + [f"- {f}" for f in text_tools("changed.txt").split()]
    lines += ["", "## Changed functions (path · name · lines)"] + [f"- {p} · {n} · {a}-{b}" for p, n, a, b in touched]
    write("changes.md", lines)
    summary.append(f"- Mode: incremental · {len(text_tools('changed.txt').split())} files, {len(touched)} functions changed (`changes.md`).")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    steps = [load_sources, section_outline, section_candidates, section_contracts, section_surface,
             section_tests, section_docs, section_scanners, section_changes]
    for step in steps:
        try:
            step()
        except Exception:
            summary.append(f"- ⚠ {step.__name__} failed: {short(traceback.format_exc(), 400)}")
    commit = text_tools("commit.txt").strip()
    write("SUMMARY.md", [f"# Facts · {commit[:8]}", ""] + summary)


main()
