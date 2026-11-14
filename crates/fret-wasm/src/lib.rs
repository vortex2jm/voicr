use fret_core::{
    builtin_presets, parse_chord as core_parse_chord, solve_chord as core_solve_chord,
    InstrumentConfig, SearchOptions,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&serializer)
        .map_err(|err| JsValue::from_str(&format!("Erro de serialização WASM: {}", err)))
}

/// Returns all built-in instrument and tuning presets.
#[wasm_bindgen]
pub fn get_presets() -> Result<JsValue, JsValue> {
    let presets = builtin_presets();
    to_js(&presets)
}

/// Parses a chord symbol and returns the normalized chord specification.
#[wasm_bindgen]
pub fn parse_chord(query: &str) -> Result<JsValue, JsValue> {
    let spec = core_parse_chord(query).map_err(|err| JsValue::from_str(&err.message))?;
    to_js(&spec)
}

/// Solves and ranks playable voicings for `query` on `instrument_js` with `options_js`.
#[wasm_bindgen]
pub fn solve_chord(
    query: &str,
    instrument_js: JsValue,
    options_js: JsValue,
) -> Result<JsValue, JsValue> {
    let spec = core_parse_chord(query).map_err(|err| JsValue::from_str(&err.message))?;

    let instrument: InstrumentConfig = serde_wasm_bindgen::from_value(instrument_js)
        .map_err(|err| JsValue::from_str(&format!("Instrumento inválido: {}", err)))?;
    instrument
        .validate()
        .map_err(|err| JsValue::from_str(&err))?;

    let options: SearchOptions = if options_js.is_null() || options_js.is_undefined() {
        SearchOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options_js)
            .map_err(|err| JsValue::from_str(&format!("Opções inválidas: {}", err)))?
    };

    let results = core_solve_chord(&spec, &instrument, &options);
    to_js(&results)
}
