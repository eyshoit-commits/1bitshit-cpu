use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn send_tg(token: &str, chat_id: &str, text: &str) -> String {
    format!("https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}", token, chat_id, text)
}
