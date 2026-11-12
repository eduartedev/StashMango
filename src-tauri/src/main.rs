#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(first) = args.next() {
        if matches!(first.as_str(), "get" | "store" | "erase") {
            stashmango_lib::credential(&first);
            return;
        }
        if first == "askpass" {
            stashmango_lib::askpass();
            return;
        }
    }
    stashmango_lib::run()
}
