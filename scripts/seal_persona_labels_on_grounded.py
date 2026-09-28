"""Attach the already-grounded parallel persona labels and reseal records."""
import hashlib
import json
from collections import OrderedDict
from pathlib import Path

SOURCE = Path(r"D:\B_Core_validation\corpora\persona_parallel_ko_v2\persona_parallel_ko_v2.jsonl")
GROUNDED = Path(r"D:\B_Core_validation\canonical_persona_parallel_ground_v1\canonical_persona_gold_grounded_ko_v1.jsonl")
OUTPUT = Path(r"D:\B_Core_validation\canonical_persona_parallel_ground_v2\canonical_persona_gold_grounded_ko_v1.jsonl")


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()


labels = {}
with SOURCE.open(encoding="utf-8") as handle:
    for line in handle:
        row = json.loads(line)
        labels[row["example_id"]] = row["persona"]["persona_label"]

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
temporary = OUTPUT.with_suffix(".tmp")
count = 0
with GROUNDED.open(encoding="utf-8") as source, temporary.open("w", encoding="utf-8", newline="\n") as target:
    for line in source:
        row = json.loads(line, object_pairs_hook=OrderedDict)
        label = labels.get(row["source_example_id"])
        if not label:
            raise RuntimeError(f"PERSONA_LABEL_MISSING:{row['source_example_id']}")
        sealed = OrderedDict()
        for key, value in row.items():
            if key == "persona_register":
                sealed["persona_label"] = label
            if key != "record_sha256":
                sealed[key] = value
        # Rust's canonical hash keeps the field in place and clears its value.
        sealed["record_sha256"] = ""
        sealed["record_sha256"] = digest(sealed)
        target.write(json.dumps(sealed, ensure_ascii=False, separators=(",", ":")) + "\n")
        count += 1
temporary.replace(OUTPUT)
receipt = {
    "schema": "BCORE.PERSONA_GROUNDED_LABEL_SEAL_RECEIPT.V1",
    "grounded_input": str(GROUNDED),
    "parallel_source": str(SOURCE),
    "record_count": count,
    "persona_label_source": "PARALLEL_GOLD_PROVENANCE_ONLY",
    "surface_authority": "UNCHANGED_NONAUTHORITATIVE_CANDIDATE",
    "output_sha256": hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),
}
receipt["receipt_sha256"] = digest(receipt)
OUTPUT.with_name("persona_grounded_label_seal_receipt_ko_v1.json").write_text(
    json.dumps(receipt, ensure_ascii=False, indent=2), encoding="utf-8"
)
print(json.dumps(receipt, ensure_ascii=False, indent=2))
