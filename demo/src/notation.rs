//! SATB staff notation, rendered by the vendored VexFlow 5 (notation.js)
//! from mokuren's existing `HarmonizationResult::to_score()`. The
//! boundary into JS is one function taking one JSON string — no
//! VexFlow-shaped type crosses back into Rust; click-to-select is read
//! straight off `data-position` attributes VexFlow's own SVG notes carry
//! after rendering; see notation.js.

use leptos::prelude::*;
use mokuren::explain::HarmonizationResult;
use mokuren::key::{Key, Mode};
use mokuren::melody::Meter;
use mokuren::pitch::Pitch;
use mokuren::voice::VoicePart;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = mokurenRenderScore)]
    fn mokuren_render_score(json: &str);
}

#[derive(serde::Serialize)]
struct NotationScore {
    key_spec: String,
    positions: Vec<NotationPosition>,
    selected_position: Option<usize>,
}

#[derive(serde::Serialize)]
struct NotationPosition {
    soprano: String,
    alto: String,
    tenor: String,
    bass: String,
    roman_numeral: String,
}

/// VexFlow's own note-key spelling: lowercase letter + accidental + "/" +
/// octave (e.g. `c/4`, `f#/3`) — mokuren's own `Accidental` display
/// (`#`, `b`, `##`, `bb`, ``) already matches VexFlow's, so no per-symbol
/// translation table is needed.
fn vexflow_pitch(pitch: Pitch) -> String {
    format!(
        "{}{}/{}",
        pitch.pitch_class.letter.to_string().to_lowercase(),
        pitch.pitch_class.accidental,
        pitch.octave.0
    )
}

/// VexFlow's key-signature spec: tonic letter+accidental, with a
/// trailing `m` for minor (e.g. `Bb`, `F#m`).
fn vexflow_key_spec(key: &Key) -> String {
    let base = format!("{}{}", key.tonic.letter, key.tonic.accidental);
    match key.mode {
        Mode::Major => base,
        Mode::Minor => format!("{base}m"),
    }
}

fn notation_json(result: &HarmonizationResult, selected_position: Option<usize>) -> String {
    let score = result.to_score(Meter::FOUR_FOUR);
    let part = |voice| {
        score
            .passage
            .parts
            .iter()
            .find(|p| p.voice == voice)
            .expect("to_score() always produces all four SATB parts")
    };
    let (soprano, alto, tenor, bass) = (
        part(VoicePart::Soprano),
        part(VoicePart::Alto),
        part(VoicePart::Tenor),
        part(VoicePart::Bass),
    );
    let positions = result
        .decisions
        .iter()
        .enumerate()
        .map(|(i, decision)| NotationPosition {
            soprano: vexflow_pitch(soprano.notes[i].pitch),
            alto: vexflow_pitch(alto.notes[i].pitch),
            tenor: vexflow_pitch(tenor.notes[i].pitch),
            bass: vexflow_pitch(bass.notes[i].pitch),
            roman_numeral: decision.selected().to_string(),
        })
        .collect();
    let notation = NotationScore {
        key_spec: vexflow_key_spec(&result.key),
        positions,
        selected_position,
    };
    serde_json::to_string(&notation).expect("NotationScore is plain data and always serializes")
}

#[component]
pub fn Notation(
    result: HarmonizationResult,
    selected_position: RwSignal<Option<usize>>,
) -> impl IntoView {
    let node_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    Effect::new(move |_| {
        if node_ref.get().is_none() {
            return;
        }
        let json = notation_json(&result, selected_position.get());
        mokuren_render_score(&json);
    });

    let on_click = move |ev: leptos::ev::MouseEvent| {
        let Some(target) = ev.target() else { return };
        let Ok(el) = target.dyn_into::<web_sys::Element>() else {
            return;
        };
        let Ok(Some(found)) = el.closest("[data-position]") else {
            return;
        };
        let Some(pos_str) = found.get_attribute("data-position") else {
            return;
        };
        if let Ok(pos) = pos_str.parse::<usize>() {
            selected_position.set(Some(pos));
        }
    };

    view! {
        <section class="notation-wrap">
            <div id="notation" node_ref=node_ref on:click=on_click></div>
        </section>
    }
}
