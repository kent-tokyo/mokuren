// Plays a harmonization back with the Web Audio API — no library, no
// sample assets, just oscillators. Rust computes all pitch/timing data
// (see notation.rs's playback_json) and calls window.mokurenPlayProgression
// with one JSON string; this file owns nothing about mokuren's data model,
// only how a note becomes a sound.

let audioCtx = null;

function ensureAudioContext() {
    if (!audioCtx) {
        audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    }
    if (audioCtx.state === 'suspended') {
        audioCtx.resume();
    }
    return audioCtx;
}

function midiToFrequency(midi) {
    return 440 * Math.pow(2, (midi - 69) / 12);
}

function scheduleNote(ctx, startTime, durationSeconds, midi) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = 'triangle';
    osc.frequency.value = midiToFrequency(midi);

    const attack = 0.02;
    const release = Math.min(0.08, durationSeconds * 0.3);
    gain.gain.setValueAtTime(0, startTime);
    gain.gain.linearRampToValueAtTime(0.18, startTime + attack);
    gain.gain.setValueAtTime(0.18, startTime + durationSeconds - release);
    gain.gain.linearRampToValueAtTime(0, startTime + durationSeconds);

    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start(startTime);
    osc.stop(startTime + durationSeconds + 0.02);
}

// Called directly from a click handler, so the AudioContext creation/
// resume above happens inside a user-gesture call stack — required by
// browser autoplay policies.
window.mokurenPlayProgression = function (json) {
    const ctx = ensureAudioContext();
    const data = JSON.parse(json);
    const beatSeconds = 60 / data.bpm;
    const startAt = ctx.currentTime + 0.05;
    data.positions.forEach((p) => {
        const start = startAt + p.start_beat * beatSeconds;
        const duration = p.duration_beats * beatSeconds;
        p.midis.forEach((midi) => scheduleNote(ctx, start, duration, midi));
    });
};
