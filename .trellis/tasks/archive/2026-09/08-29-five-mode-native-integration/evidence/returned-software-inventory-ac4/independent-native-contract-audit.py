import collections
import json
import re
from pathlib import Path

ev = Path(__file__).resolve().parent
raw = (ev / "independent-native-inventory-run1.json").read_text(encoding="utf-8")
doc = json.loads(raw)
data = doc["data"]
entries = data["entries"]
sources = data["sources"]

arp = sum(1 for e in entries if e["identity"]["source"] == "arp")
msi = sum(1 for e in entries if e["identity"]["source"] == "msi")
msix = sum(1 for e in entries if e["identity"]["source"] == "msix")
msi_non_manual = [
    e
    for e in entries
    if e["identity"]["source"] == "msi" and e["eligibility"]["state"] != "manual"
]
sel_non_msix = [
    e
    for e in entries
    if e["identity"]["source"] != "msix" and e["eligibility"]["state"] == "selectable"
]
last_used_bad = [
    e
    for e in entries
    if e["last_used"]
    != {"state": "unknown", "reason_code": "no_supported_exact_source"}
]
msix_non_cu = [
    e
    for e in entries
    if e["identity"]["source"] == "msix" and e.get("scope") != "current_user"
]
path_props = [e for e in entries if "path" in e or "installed_path" in e]
size_codes = sorted(
    {
        e["size"]["source_code"]
        for e in entries
        if "source_code" in e["size"]
    }
)
size_states = dict(collections.Counter(e["size"]["state"] for e in entries))
allowed_size_codes = {
    "arp_estimated_size_kib",
    "msi_estimated_size_kib",
    "msix_installed_path",
}
unexpected_codes = sorted(set(size_codes) - allowed_size_codes)
last_used_keys = sorted({tuple(sorted(e["last_used"].keys())) for e in entries})
ident_keys = sorted({tuple(sorted(e["identity"].keys())) for e in entries})
fingerprint = data.get("fingerprint", "")
forbidden = {
    "UninstallString": "UninstallString" in raw,
    "QuietUninstallString": "QuietUninstallString" in raw,
    "DisplayIcon": "DisplayIcon" in raw,
    "quoted_path_key": bool(re.search(r'"path"\s*:', raw)),
}
source_states = []
for s in sources:
    sid = s["source"]
    src = sid.get("source")
    if "hive" in sid:
        label = f"{src}/{s['state']} hive={sid['hive']} view={sid['view']}"
    elif "context" in sid:
        label = f"{src}/{s['state']} context={sid['context']}"
    else:
        label = f"{src}/{s['state']}"
    source_states.append(label)

sel = [e for e in entries if e["eligibility"]["state"] == "selectable"]
sel_reasons = dict(collections.Counter(e["eligibility"]["reason"] for e in sel))
sel_sources = dict(collections.Counter(e["identity"]["source"] for e in sel))

contract_ok = all(
    [
        len(msi_non_manual) == 0,
        len(sel_non_msix) == 0,
        len(last_used_bad) == 0,
        len(msix_non_cu) == 0,
        len(path_props) == 0,
        not forbidden["UninstallString"],
        not forbidden["QuietUninstallString"],
        not forbidden["DisplayIcon"],
        not forbidden["quoted_path_key"],
        not unexpected_codes,
        arp + msi + msix == len(entries),
    ]
)

snap = {
    "envelope_outcome": doc.get("outcome"),
    "command": doc.get("command"),
    "source_count": len(sources),
    "entry_count": len(entries),
    "arp": arp,
    "msi": msi,
    "msix": msix,
    "msi_non_manual": len(msi_non_manual),
    "selectable_non_msix": len(sel_non_msix),
    "selectable_count": len(sel),
    "selectable_reasons": sel_reasons,
    "selectable_sources": sel_sources,
    "last_used_not_exact_unknown": len(last_used_bad),
    "last_used_key_sets": [list(k) for k in last_used_keys],
    "msix_non_current_user": len(msix_non_cu),
    "entries_with_path_property": len(path_props),
    "identity_key_sets": [list(k) for k in ident_keys],
    "forbidden": forbidden,
    "size_source_codes": size_codes,
    "unexpected_size_source_codes": unexpected_codes,
    "size_states": size_states,
    "source_states": source_states,
    "fingerprint_prefix": fingerprint[:20],
    "inventory_version": data.get("version"),
    "contract_ok": contract_ok,
}
(ev / "independent-native-contract-snapshot.json").write_text(
    json.dumps(snap, indent=2) + "\n",
    encoding="utf-8",
)
print(json.dumps(snap, indent=2))
print("CONTRACT_OK", contract_ok)
