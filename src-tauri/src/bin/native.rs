// 100% Native Rust & Slint UI entry point for Partisipasi Sekolah v0.2.0
// Runs without WebView2 / Chromium engine, optimized for 4GB RAM PCs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    partisipasi_sekolah_lib::run_slint_native()
}
