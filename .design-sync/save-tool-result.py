# Saves a DesignSync get_file result persisted by Claude Code (JSON) to a file.
# Usage: python3 save-tool-result.py <result.txt> <destination>
import json, sys, base64
raw = open(sys.argv[1]).read()
data = json.loads(raw[raw.index('{'):])
content = data['content']
mode = 'wb' if data.get('isBase64') else 'w'
with open(sys.argv[2], mode) as f:
    f.write(base64.b64decode(content) if data.get('isBase64') else content)
print(sys.argv[2], len(content))
