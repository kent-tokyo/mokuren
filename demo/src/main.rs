mod i18n;

use i18n::use_lang;
use leptos::prelude::*;
use mokuren::key::Mode;
use mokuren::melody::Position;
use mokuren::prelude::*;

const DEFAULT_MELODY: &str = "C4 C4 G4 G4 A4 A4 G4";

fn harmonize(
    melody_text: &str,
    tonic_text: &str,
    mode: Mode,
    width: usize,
) -> Result<HarmonizationResult> {
    let melody = Melody::parse(melody_text)?;
    let tonic = tonic_text
        .parse()
        .map_err(|_| MokurenError::Parse(format!("bad key {tonic_text:?}")))?;
    let key = Key::new(tonic, mode)?;
    Composer::new()
        .key(key)
        .style(Style::CommonPractice)
        .search(BeamSearch::new().width(width))
        .harmonize(melody)
}

#[component]
fn AlternativesPanel(result: HarmonizationResult, position: usize) -> impl IntoView {
    let lang = use_lang();
    let decision = result.decisions[position].clone();
    let selected = decision.selected();
    let why = result.why(Position::new(position)).unwrap_or_default();

    let mut alternatives: Vec<_> = decision.evaluated().to_vec();
    alternatives.sort_by(mokuren::generate::compare_candidates);

    let selected_why_not = RwSignal::new(None::<RomanNumeral>);

    view! {
        <section class="panel">
            <h3>{move || i18n::position_label(lang.get())} " " {position} ": " {selected.to_string()}</h3>
            <p class="note">{move || i18n::why_note(lang.get())}</p>
            <pre>{why}</pre>

            <h4>{move || i18n::alternatives_header(lang.get())}</h4>
            <ul class="alternatives">
                {alternatives
                    .into_iter()
                    .map(|c| {
                        let rn = c.roman_numeral;
                        let is_selected = rn == selected;
                        let is_valid = c.is_valid();
                        let score = c.score.total();
                        let status = move || {
                            if is_selected {
                                i18n::status_selected(lang.get())
                            } else if is_valid {
                                i18n::status_valid(lang.get())
                            } else {
                                i18n::status_rejected(lang.get())
                            }
                        };
                        view! {
                            <li>
                                <button
                                    class:selected=is_selected
                                    title=move || i18n::alternative_tip(lang.get())
                                    on:click=move |_| selected_why_not.set(Some(rn))
                                >
                                    {rn.to_string()} " — " {status} " (" {format!("{score:+.2}")} ")"
                                </button>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>

            {move || {
                selected_why_not
                    .get()
                    .map(|alt| {
                        let text = result
                            .why_not(Position::new(position), alt)
                            .unwrap_or_else(|e| e.to_string());
                        view! {
                            <div class="why-not">
                                <pre>{text}</pre>
                            </div>
                        }
                    })
            }}
        </section>
    }
}

const RACE_WIDTHS: [usize; 7] = [4, 8, 16, 32, 64, 128, 256];

#[component]
fn BeamSearchRace(
    melody_text: RwSignal<String>,
    tonic_text: RwSignal<String>,
    mode: RwSignal<Mode>,
) -> impl IntoView {
    let lang = use_lang();
    let rows = RwSignal::new(Vec::<(usize, bool, u64, f64)>::new());
    let running = RwSignal::new(false);

    let run_race = move |_| {
        rows.set(Vec::new());
        running.set(true);
        let melody = melody_text.get_untracked();
        let tonic = tonic_text.get_untracked();
        let mode = mode.get_untracked();
        leptos::task::spawn_local(async move {
            for &w in &RACE_WIDTHS {
                // Yield to the browser between widths so the table (and
                // the "running" state) actually repaints as it goes,
                // instead of the whole race running as one blocking
                // synchronous stretch — WASM has no threads here, so
                // without this the tab would look frozen until every
                // width finished.
                gloo_timers::future::TimeoutFuture::new(0).await;
                let start = js_sys::Date::now();
                let outcome = harmonize(&melody, &tonic, mode, w);
                let elapsed_ms = js_sys::Date::now() - start;
                let (success, candidates) = match &outcome {
                    Ok(result) => (true, result.diagnostics().candidates_generated),
                    Err(_) => (false, 0),
                };
                rows.update(|r| r.push((w, success, candidates, elapsed_ms)));
            }
            running.set(false);
        });
    };

    view! {
        <section class="race" title=move || i18n::beam_race_tip(lang.get())>
            <h2>{move || i18n::beam_race_header(lang.get())}</h2>
            <button on:click=run_race disabled=move || running.get()>
                {move || i18n::beam_race_button(lang.get())}
            </button>
            {move || {
                (!rows.get().is_empty())
                    .then(|| {
                        view! {
                            <table>
                                <thead>
                                    <tr>
                                        <th>{move || i18n::beam_race_width_col(lang.get())}</th>
                                        <th>{move || i18n::beam_race_result_col(lang.get())}</th>
                                        <th>{move || i18n::beam_race_candidates_col(lang.get())}</th>
                                        <th>{move || i18n::beam_race_runtime_col(lang.get())}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows
                                        .get()
                                        .into_iter()
                                        .map(|(w, success, candidates, ms)| {
                                            let status = if success {
                                                i18n::beam_race_success(lang.get())
                                            } else {
                                                i18n::beam_race_fail(lang.get())
                                            };
                                            view! {
                                                <tr class:race-fail=!success>
                                                    <td>{w}</td>
                                                    <td>{status}</td>
                                                    <td>{candidates}</td>
                                                    <td>{format!("{ms:.1}")}</td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        }
                    })
            }}
        </section>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Compose,
    Search,
    Explain,
}

#[component]
fn App() -> impl IntoView {
    let lang = i18n::provide_lang();
    let melody_text = RwSignal::new(DEFAULT_MELODY.to_string());
    let tonic_text = RwSignal::new("C".to_string());
    let mode = RwSignal::new(Mode::Major);
    let width = RwSignal::new(32usize);
    let result = RwSignal::new(None::<std::result::Result<HarmonizationResult, String>>);
    let selected_position = RwSignal::new(None::<usize>);
    let active_tab = RwSignal::new(Tab::Compose);

    let run = move || {
        let outcome = harmonize(
            &melody_text.get(),
            &tonic_text.get(),
            mode.get(),
            width.get(),
        )
        .map_err(|e| e.to_string());
        result.set(Some(outcome));
        selected_position.set(None);
    };
    // Harmonize the default melody immediately so the page isn't empty
    // on first load.
    run();

    view! {
        <main>
            <div class="header-row">
                <div>
                    <h1>"mokuren"</h1>
                    <p class="tagline">{move || i18n::tagline(lang.get())}</p>
                </div>
                <div class="lang-select">
                    <svg
                        class="lang-icon"
                        xmlns="http://www.w3.org/2000/svg"
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <circle cx="12" cy="12" r="10"></circle>
                        <line x1="2" y1="12" x2="22" y2="12"></line>
                        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                    </svg>
                    <select
                        class="lang-toggle"
                        on:change=move |ev| lang.set(i18n::Lang::from_label(&event_target_value(&ev)))
                    >
                        <option value=i18n::Lang::En.label() selected=move || lang.get() == i18n::Lang::En>
                            {i18n::Lang::En.label()}
                        </option>
                        <option value=i18n::Lang::Ja.label() selected=move || lang.get() == i18n::Lang::Ja>
                            {i18n::Lang::Ja.label()}
                        </option>
                    </select>
                </div>
            </div>

            <section class="input">
                <label title=move || i18n::melody_tip(lang.get())>
                    {move || i18n::melody_label(lang.get())} " "
                    <input
                        type="text"
                        prop:value=move || melody_text.get()
                        on:input=move |ev| melody_text.set(event_target_value(&ev))
                    />
                </label>
                <label title=move || i18n::key_tip(lang.get())>
                    {move || i18n::key_label(lang.get())} " "
                    <input
                        type="text"
                        size="3"
                        prop:value=move || tonic_text.get()
                        on:input=move |ev| tonic_text.set(event_target_value(&ev))
                    />
                </label>
                <label title=move || i18n::mode_tip(lang.get())>
                    <select on:change=move |ev| {
                        mode.set(if event_target_value(&ev) == "minor" { Mode::Minor } else { Mode::Major })
                    }>
                        <option value="major">{move || i18n::mode_major(lang.get())}</option>
                        <option value="minor">{move || i18n::mode_minor(lang.get())}</option>
                    </select>
                </label>
                <label title=move || i18n::beam_width_tip(lang.get())>
                    {move || i18n::beam_width_label(lang.get())} " "
                    <input
                        type="number"
                        prop:value=move || width.get().to_string()
                        on:input=move |ev| {
                            if let Ok(w) = event_target_value(&ev).parse() {
                                width.set(w);
                            }
                        }
                    />
                </label>
                <button on:click=move |_| run()>{move || i18n::harmonize_button(lang.get())}</button>
            </section>

            <div class="tabs">
                <button
                    class:active=move || active_tab.get() == Tab::Compose
                    on:click=move |_| active_tab.set(Tab::Compose)
                >
                    {move || i18n::harmonization_header(lang.get())}
                </button>
                <button
                    class:active=move || active_tab.get() == Tab::Search
                    on:click=move |_| active_tab.set(Tab::Search)
                >
                    {move || i18n::beam_race_header(lang.get())}
                </button>
                <button
                    class:active=move || active_tab.get() == Tab::Explain
                    on:click=move |_| active_tab.set(Tab::Explain)
                >
                    {move || i18n::explain_tab_label(lang.get())}
                </button>
            </div>

            {move || {
                if active_tab.get() != Tab::Compose {
                    return None;
                }
                result
                    .get()
                    .map(|outcome| match outcome {
                        Ok(result) => {
                            let progression: Vec<_> = result
                                .decisions
                                .iter()
                                .enumerate()
                                .map(|(i, d)| (i, d.selected().to_string()))
                                .collect();
                            let diag = result.diagnostics().clone();
                            let result_for_panel = result.clone();
                            view! {
                                <section class="progression" title=move || i18n::progression_tip(lang.get())>
                                    <h2>{move || i18n::harmonization_header(lang.get())}</h2>
                                    <ul>
                                        {progression
                                            .into_iter()
                                            .map(|(i, label)| {
                                                view! {
                                                    <li>
                                                        <button on:click=move |_| selected_position.set(Some(i))>
                                                            {label}
                                                        </button>
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                    <p class="diagnostics" title=move || i18n::diagnostics_tip(lang.get())>
                                        {move || i18n::generated_label(lang.get())} " " {diag.candidates_generated}
                                        " · " {move || i18n::retained_label(lang.get())} " " {diag.candidates_retained}
                                        " · " {move || i18n::rejected_label(lang.get())} " " {diag.candidates_rejected}
                                    </p>
                                </section>
                                {move || {
                                    selected_position
                                        .get()
                                        .map(|pos| view! { <AlternativesPanel result=result_for_panel.clone() position=pos /> })
                                }}
                            }
                                .into_any()
                        }
                        Err(e) => {
                            let message = move || format!("{}{e}", i18n::error_prefix(lang.get()));
                            view! { <p class="error">{message}</p> }.into_any()
                        }
                    })
            }}

            {move || {
                (active_tab.get() == Tab::Search)
                    .then(|| view! { <BeamSearchRace melody_text=melody_text tonic_text=tonic_text mode=mode /> })
            }}

            {move || {
                (active_tab.get() == Tab::Explain)
                    .then(|| {
                        view! {
                            <section class="explain">
                                <h2>{move || i18n::explain_tab_label(lang.get())}</h2>
                                <p>{move || i18n::explain_intro(lang.get())}</p>

                                <h3>{move || i18n::harmonization_header(lang.get())}</h3>
                                <p>{move || i18n::explain_compose_body(lang.get())}</p>

                                <h3>{move || i18n::beam_race_header(lang.get())}</h3>
                                <p>{move || i18n::explain_search_body_1(lang.get())}</p>
                                <p>{move || i18n::explain_search_body_2(lang.get())}</p>
                                <dl>
                                    <dt>{move || i18n::beam_race_width_col(lang.get())}</dt>
                                    <dd>{move || i18n::explain_width(lang.get())}</dd>
                                    <dt>{move || i18n::beam_race_candidates_col(lang.get())}</dt>
                                    <dd>{move || i18n::explain_candidates(lang.get())}</dd>
                                    <dt>{move || i18n::beam_race_runtime_col(lang.get())}</dt>
                                    <dd>{move || i18n::explain_runtime(lang.get())}</dd>
                                </dl>
                            </section>
                        }
                    })
            }}
        </main>
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
