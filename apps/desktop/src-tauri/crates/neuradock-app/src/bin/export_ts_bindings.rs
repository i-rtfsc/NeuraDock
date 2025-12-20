use anyhow::Context;
use specta_typescript::{BigIntExportBehavior, Typescript};

fn main() -> anyhow::Result<()> {
    let out_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/lib/tauri.ts");

    std::fs::create_dir_all(out_path.parent().context("tauri.ts has no parent dir")?)
        .context("create apps/desktop/src/lib directory")?;

    let exporter = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .header("// eslint-disable\n// @ts-nocheck\n");

    neuradock_app_lib::presentation::ipc::builder()
        .export(exporter, &out_path)
        .context("export tauri-specta TypeScript bindings")?;

    // Post-process: prevent TS6133 on unused generated imports.
    let mut generated = std::fs::read_to_string(&out_path).context("read generated tauri.ts")?;
    if generated.contains("Channel as TAURI_CHANNEL") && !generated.contains("void TAURI_CHANNEL") {
        let import_end = "} from \"@tauri-apps/api/core\";\n";
        if let Some(idx) = generated.find(import_end) {
            let insert_at = idx + import_end.len();
            generated.insert_str(insert_at, "void TAURI_CHANNEL;\n");
            std::fs::write(&out_path, generated).context("write post-processed tauri.ts")?;
        }
    }

    println!("Generated {}", out_path.display());
    Ok(())
}
