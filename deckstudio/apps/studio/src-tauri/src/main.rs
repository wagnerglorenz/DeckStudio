#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::WindowEvent;

fn main() {
    tauri::Builder::default()
        .on_window_event(|_window, event| {
            // Encerra o processo do editor por completo ao fechar a janela (0 MB WebView2 em idle).
            if let WindowEvent::CloseRequested { .. } = event {
                std::process::exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("Erro ao iniciar DeckStudio");
}
