// Thin glue between mokuren's WASM module and the vendored VexFlow 5
// engraving library (vexflow.js, loaded as a plain <script> before this
// file — see index.html). mokuren itself carries no VexFlow dependency
// (see src/explain.rs's HarmonizationResult::to_score()); this is the
// only place VexFlow-specific vocabulary (duration codes, key-signature
// spelling, stem direction) exists. Rust calls window.mokurenRenderScore
// with one JSON string; nothing VexFlow-shaped crosses back into Rust.
//
// Click-to-select needs no JS -> Rust callback: each rendered notehead
// gets a data-position attribute, and the Rust side listens for clicks
// on the container directly (Element.closest("[data-position]")).

const fontsReady = window.VexFlow.loadFonts('Bravura', 'Academico');

function clearContainer(container) {
    while (container.firstChild) {
        container.removeChild(container.firstChild);
    }
}

window.mokurenRenderScore = async function (json) {
    await fontsReady;

    const VF = window.VexFlow;
    const container = document.getElementById('notation');
    if (!container) {
        return;
    }
    clearContainer(container);

    const data = JSON.parse(json);
    const noteWidth = 70;
    const width = Math.max(container.clientWidth || 600, 120 + data.positions.length * noteWidth);

    const factory = new VF.Factory({
        renderer: { elementId: 'notation', width, height: 280 },
    });
    const system = factory.System({ x: 10, y: 10, width: width - 20 });

    const buildNotes = (voicePart, clef, stemDirection) =>
        data.positions.map((p, i) => {
            const note = factory.StaveNote({
                keys: [p[voicePart]],
                duration: 'q',
                clef,
                stemDirection,
            });
            if (i === data.selected_position) {
                note.setStyle({ fillStyle: '#6a4fb6', strokeStyle: '#6a4fb6' });
            }
            return note;
        });

    const sopranoNotes = buildNotes('soprano', 'treble', VF.Stem.UP);
    const altoNotes = buildNotes('alto', 'treble', VF.Stem.DOWN);
    const tenorNotes = buildNotes('tenor', 'bass', VF.Stem.UP);
    const bassNotes = buildNotes('bass', 'bass', VF.Stem.DOWN);

    bassNotes.forEach((note, i) => {
        const annotation = factory.Annotation({
            text: data.positions[i].roman_numeral,
            vJustify: 'below',
        });
        note.addModifier(annotation, 0);
    });

    const makeVoice = (notes) => {
        const voice = factory.Voice({ time: '4/4' });
        voice.setMode(VF.Voice.Mode.SOFT);
        voice.addTickables(notes);
        return voice;
    };

    const sopranoVoice = makeVoice(sopranoNotes);
    const altoVoice = makeVoice(altoNotes);
    const tenorVoice = makeVoice(tenorNotes);
    const bassVoice = makeVoice(bassNotes);

    system
        .addStave({ voices: [sopranoVoice, altoVoice] })
        .addClef('treble')
        .addKeySignature(data.key_spec);
    system
        .addStave({ voices: [tenorVoice, bassVoice] })
        .addClef('bass')
        .addKeySignature(data.key_spec);
    system.addConnector('brace');

    VF.Accidental.applyAccidentals([sopranoVoice, altoVoice, tenorVoice, bassVoice], data.key_spec);

    factory.draw();

    [sopranoNotes, altoNotes, tenorNotes, bassNotes].forEach((notes) => {
        notes.forEach((note, i) => {
            const el = note.getSVGElement();
            if (el) {
                el.setAttribute('data-position', String(i));
            }
        });
    });
};
