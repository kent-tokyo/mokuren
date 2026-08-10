#!/usr/bin/env python3
"""Extracts Bach chorales from a local music21 install into mokuren's
`.chorale` v2 fixture format (BENCHMARK.md), for
`examples/chorale_benchmark.rs`.

Decided as the canonical external source (BENCHMARK.md, 2026-08-10):
music21's own docs state Margaret Greentree "kindly gave permission for
distribution of her edited collection of the Bach chorales in MusicXML
format as part of the music21 corpus" — which is exactly the context
this script stays inside: it only ever reads *your own local* music21
install and writes fixture files to a directory *you* choose. This
repository never vendors the output. Nothing this script writes should
be committed to mokuren's repository.

Requires: pip install music21 (with its bundled corpus — no network
access needed at extraction time).

Usage:
    python3 tools/music21_chorale_extractor.py --major-only -o /path/to/output

What it does NOT do, on purpose:
  - It does not build an independent onset grid for alto/tenor/bass.
    Reference voices are sampled *at each soprano onset only* — giving
    them their own timing would leak where Bach changed harmony (the
    thing a benchmark run is supposed to discover) into the input data.
  - It does not silently round an unrepresentable duration. A chorale
    whose rhythm doesn't reduce to mokuren's Duration vocabulary
    (whole/dotted-half/half/dotted-quarter/quarter/dotted-eighth/eighth/
    sixteenth — see src/melody.rs) is skipped with a clear reason, not
    approximated.
  - A soprano rest is written out as a `REST` event in the fixture
    (fixture format v3) rather than skipping the chorale — mokuren's
    Composer::harmonize still only ever sees a plain, rest-free Melody:
    the harness splits at each rest into independent phrases before
    harmonizing (see examples/chorale_benchmark.rs's MelodyLine::phrases
    usage). Alto/tenor/bass reference pitches are still sampled only at
    soprano *note* onsets — a rest has no onset to sample against.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

try:
    import music21
    from music21 import corpus
except ImportError:
    print("music21 is required: pip install music21", file=sys.stderr)
    sys.exit(1)

# quarterLength -> mokuren duration fraction (n/d of a whole note).
# Anything not in this table is unrepresentable in mokuren's Duration
# (src/melody.rs) and causes the chorale to be skipped, not rounded.
QUARTER_LENGTH_TO_FRACTION = {
    4.0: "1/1",
    3.0: "3/4",
    2.0: "1/2",
    1.5: "3/8",
    1.0: "1/4",
    0.75: "3/16",
    0.5: "1/8",
    0.25: "1/16",
}


def to_mokuren_pitch(note) -> str:
    """music21 uses '-' for flat (e.g. 'B-4'); mokuren uses 'b' (e.g.
    'Bb4'). Both use '#' for sharp, so only flats need translating."""
    name = note.nameWithOctave
    return name.replace("-", "b")


def sounding_at(stream, offset):
    """The note/rest whose span covers `offset`, or None past the end."""
    for n in stream:
        if n.offset <= offset < n.offset + n.duration.quarterLength:
            return n
    return None


def extract_chorale(chorale):
    """Returns (fixture_text, warning) — warning is None on success, or
    a human-readable reason the chorale was skipped (fixture_text is
    None in that case)."""
    parts = {p.partName: p for p in chorale.parts}
    for required in ("Soprano", "Alto", "Tenor", "Bass"):
        if required not in parts:
            return None, f"missing {required} part"

    soprano = list(parts["Soprano"].flatten().notesAndRests)
    alto = list(parts["Alto"].flatten().notesAndRests)
    tenor = list(parts["Tenor"].flatten().notesAndRests)
    bass = list(parts["Bass"].flatten().notesAndRests)

    if all(n.isRest for n in soprano):
        return None, "soprano is nothing but rests"

    key = chorale.analyze("key")
    if key.mode != "major":
        return None, f"mode is {key.mode!r}, not major (mokuren doesn't support minor yet)"

    time_sigs = chorale.parts[0].flatten().getElementsByClass("TimeSignature")
    meter = time_sigs[0].ratioString if time_sigs else "4/4"

    soprano_lines = []
    reference_alto, reference_tenor, reference_bass = [], [], []
    for n in soprano:
        ql = float(n.duration.quarterLength)
        fraction = QUARTER_LENGTH_TO_FRACTION.get(ql)
        if fraction is None:
            return None, f"soprano has an unrepresentable duration ({ql} quarter-lengths) at offset {n.offset}"

        if n.isRest:
            soprano_lines.append(f"{n.offset} REST {fraction}")
            continue

        soprano_lines.append(f"{n.offset} {to_mokuren_pitch(n)} {fraction}")

        for ref_stream, ref_list, label in (
            (alto, reference_alto, "alto"),
            (tenor, reference_tenor, "tenor"),
            (bass, reference_bass, "bass"),
        ):
            sounding = sounding_at(ref_stream, n.offset)
            if sounding is None or sounding.isRest:
                return None, f"{label} has no sounding note at soprano offset {n.offset}"
            ref_list.append(to_mokuren_pitch(sounding))

    tonic = key.tonic.name.replace("-", "b")
    riemenschneider = chorale.metadata.number
    title = chorale.metadata.title or f"Riemenschneider {riemenschneider}"

    fixture = "\n".join(
        [
            f"name: {title} (Riemenschneider {riemenschneider})",
            f"key: {tonic}",
            f"meter: {meter}",
            "soprano:",
            *soprano_lines,
            f"alto: {' '.join(reference_alto)}",
            f"tenor: {' '.join(reference_tenor)}",
            f"bass: {' '.join(reference_bass)}",
            "",
        ]
    )
    return fixture, None


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-o", "--output", required=True, type=Path, help="output directory for .chorale files")
    parser.add_argument("--major-only", action="store_true", default=True, help="skip minor-mode chorales (default: on, since mokuren doesn't support minor yet)")
    parser.add_argument("--limit", type=int, default=None, help="stop after this many successfully extracted chorales")
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)

    extracted = []
    skipped = []
    iterator = corpus.chorales.Iterator(numberingSystem="riemenschneider")
    for chorale in iterator:
        number = chorale.metadata.number
        fixture_text, warning = extract_chorale(chorale)
        if fixture_text is None:
            skipped.append({"riemenschneider": number, "reason": warning})
            continue

        file_hash = hashlib.sha256(fixture_text.encode("utf-8")).hexdigest()
        # Riemenschneider numbers are strings and occasionally have a
        # letter suffix for a variant setting (e.g. "150a"); zero-pad
        # only the purely numeric case for readable sorting.
        padded_number = number.zfill(3) if number.isdigit() else number
        out_path = args.output / f"riemenschneider-{padded_number}.chorale"
        out_path.write_text(fixture_text)
        extracted.append({"riemenschneider": number, "file": out_path.name, "sha256": file_hash})

        if args.limit and len(extracted) >= args.limit:
            break

    manifest = {
        "source": "music21",
        "music21_version": music21.__version__,
        "numbering": "riemenschneider",
        "adapter_version": "1",
        "adapter_script": "tools/music21_chorale_extractor.py",
        "major_only": args.major_only,
        "extracted_count": len(extracted),
        "skipped_count": len(skipped),
        "selected_ids": [e["riemenschneider"] for e in extracted],
        "source_file_hashes": {e["file"]: e["sha256"] for e in extracted},
        "skipped": skipped,
    }
    manifest_path = args.output / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))

    print(f"extracted {len(extracted)} chorale(s) to {args.output}")
    print(f"skipped {len(skipped)} chorale(s) — see {manifest_path} for reasons")
    print(f"manifest: {manifest_path}")


if __name__ == "__main__":
    main()
