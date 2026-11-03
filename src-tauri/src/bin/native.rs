// Entry point for Partisipasi Sekolah Desktop v0.2.0
// Runs without WebView / Chromium engine, optimized for performance and low memory.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    partisipasi_sekolah_lib::run_slint_native()
}
