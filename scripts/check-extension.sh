#!/usr/bin/env bash
# Static checks for the GNOME Shell extension.
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# The same script CI runs. Nothing here needs a running shell, so it is also
# the fastest way to catch a mistake locally before a session restart, which is
# the only way to load extension code under Wayland.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXT="${ROOT}/extension"
FAILED=0

fail() { printf '  \033[31mFAIL\033[0m %s\n' "$1"; FAILED=1; }
pass() { printf '  \033[32mok\033[0m   %s\n' "$1"; }

echo "Syntax"
while IFS= read -r file; do
  if node --check "${file}" 2>/dev/null; then pass "${file#"${EXT}/"}"
  else fail "${file#"${EXT}/"} does not parse"; fi
done < <(find "${EXT}" -name '*.js' | sort)

echo "Metadata"
META="${EXT}/metadata.json"
if python3 -c "import json,sys; json.load(open('${META}'))" 2>/dev/null; then
  pass "metadata.json is valid JSON"
else
  fail "metadata.json is not valid JSON"
fi
python3 - "${META}" "${EXT}" <<'PY' || FAILED=1
import json, pathlib, re, sys
meta = json.loads(pathlib.Path(sys.argv[1]).read_text())
ext = pathlib.Path(sys.argv[2])
bad = False

for field in ("uuid", "name", "description", "shell-version", "url"):
    if not meta.get(field):
        print(f"  \033[31mFAIL\033[0m metadata.json is missing {field}"); bad = True

uuid = meta.get("uuid", "")
if not re.fullmatch(r"[A-Za-z0-9._-]+@[A-Za-z0-9._-]+", uuid):
    print(f"  \033[31mFAIL\033[0m uuid {uuid!r} is not id@namespace"); bad = True
elif uuid.endswith("gnome.org"):
    print("  \033[31mFAIL\033[0m uuid may not use the gnome.org namespace"); bad = True
else:
    print(f"  \033[32mok\033[0m   uuid {uuid}")

# A declared schema that does not exist fails only at runtime, in a process the
# developer cannot easily restart, so it is worth catching here.
schema = meta.get("settings-schema")
if schema:
    sources = list((ext / "schemas").glob("*.gschema.xml"))
    ids = {m for s in sources for m in re.findall(r'schema id="([^"]+)"', s.read_text())}
    if schema in ids:
        print(f"  \033[32mok\033[0m   settings-schema {schema} is defined")
    else:
        print(f"  \033[31mFAIL\033[0m settings-schema {schema} is declared but not defined"); bad = True

sys.exit(1 if bad else 0)
PY

echo "Process isolation"
# Shell-process code may not touch the toolkit libraries, and preferences code
# may not touch the shell's own. Both crash the host process rather than fail.
if grep -rlE "gi://(Gtk|Adw|Gdk)" "${EXT}/extension.js" "${EXT}/lib" 2>/dev/null | grep -q .; then
  fail "toolkit library imported into the shell process"
else
  pass "no Gtk, Adw or Gdk in the shell process"
fi
if grep -lE "gi://(St|Clutter|Meta|Shell)" "${EXT}/prefs.js" 2>/dev/null | grep -q .; then
  fail "shell library imported into the preferences process"
else
  pass "no St, Clutter, Meta or Shell in preferences"
fi

echo "Deprecated modules"
if grep -rnE "imports\.(mainloop|lang|byteArray)|from 'gi://ByteArray'" "${EXT}" 2>/dev/null | grep -q .; then
  fail "Mainloop, Lang or ByteArray is still used"
else
  pass "no Mainloop, Lang or ByteArray"
fi

echo "Settings keys"
python3 - "${EXT}" <<'PY' || FAILED=1
import pathlib, re, sys
ext = pathlib.Path(sys.argv[1])
declared = set()
for source in (ext / "schemas").glob("*.gschema.xml"):
    declared |= set(re.findall(r'<key name="([^"]+)"', source.read_text()))

used = set()
for source in ext.rglob("*.js"):
    text = source.read_text()
    used |= set(re.findall(r"get_(?:boolean|int|string|double)\('([^']+)'\)", text))
    used |= set(re.findall(r"settings\.bind\('([^']+)'", text))
    # Keys reached indirectly, through the declarative cell and toggle tables.
    used |= set(re.findall(r"\bkey: '([a-z][a-z0-9-]*)'", text))

missing = sorted(used - declared)
for key in missing:
    print(f"  \033[31mFAIL\033[0m setting {key!r} is read but not declared")
if not missing:
    print(f"  \033[32mok\033[0m   all {len(used)} settings read are declared")
sys.exit(1 if missing else 0)
PY

echo "Schema compiles"
if glib-compile-schemas --strict --dry-run "${EXT}/schemas" 2>/dev/null; then
  pass "gschema is valid"
else
  fail "gschema does not compile"
fi

exit "${FAILED}"
