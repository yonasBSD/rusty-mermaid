#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Parser must never panic on any input — only Ok or Err.
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = rusty_mermaid_diagrams::quadrant::parser::parse(input);
    }
});
