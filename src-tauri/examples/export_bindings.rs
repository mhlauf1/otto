//! `pnpm bindings` — regenerate `src/bindings.ts` without launching the app.

fn main() {
    if let Err(err) = otto_lib::export_bindings(otto_lib::bindings_path()) {
        eprintln!("failed to export bindings: {err}");
        std::process::exit(1);
    }
    println!("wrote {}", otto_lib::bindings_path().display());
}
