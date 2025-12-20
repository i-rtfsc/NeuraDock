use anyhow::Context;
use specta_typescript::{BigIntExportBehavior, Typescript};

fn main() -> anyhow::Result<()> {
    let out_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src/lib/tauri.ts");

    std::fs::create_dir_all(out_path.parent().context("tauri.ts has no parent dir")?)
        .context("create apps/desktop/src/lib directory")?;

    let exporter = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .header("// eslint-disable\n");

    neuradock_app_lib::presentation::ipc::builder()
        .export(exporter, &out_path)
        .context("export tauri-specta TypeScript bindings")?;

    // Post-process: keep generated output strict-TS friendly.
    let mut generated = std::fs::read_to_string(&out_path).context("read generated tauri.ts")?;

    // Prevent TS6133 on unused generated imports.
    if generated.contains("Channel as TAURI_CHANNEL") && !generated.contains("void TAURI_CHANNEL") {
        let import_end = "} from \"@tauri-apps/api/core\";\n";
        if let Some(idx) = generated.find(import_end) {
            let insert_at = idx + import_end.len();
            generated.insert_str(insert_at, "void TAURI_CHANNEL;\n");
        }
    }

    // Avoid `as any` for command errors; normalize unknown error payloads.
    if !generated.contains("function __coerceCommandError(") {
        let anchor = "| { status: \"error\"; error: E };\n\n";
        if let Some(idx) = generated.find(anchor) {
            let insert_at = idx + anchor.len();
            generated.insert_str(
                insert_at,
                "function __coerceCommandError(error: unknown): CommandError {\n\tif (error && typeof error === \"object\") {\n\t\tconst maybe = error as Partial<CommandError>;\n\t\tif (\n\t\t\ttypeof maybe.code === \"number\" &&\n\t\t\ttypeof maybe.message === \"string\" &&\n\t\t\ttypeof maybe.severity === \"string\" &&\n\t\t\ttypeof maybe.recoverable === \"boolean\"\n\t\t) {\n\t\t\treturn maybe as CommandError;\n\t\t}\n\t\tconst wrapped = error as { error?: unknown };\n\t\tif (wrapped.error) return __coerceCommandError(wrapped.error);\n\t}\n\tif (typeof error === \"string\") {\n\t\treturn { code: 5001, message: error, severity: \"Error\", recoverable: false };\n\t}\n\treturn {\n\t\tcode: 5001,\n\t\tmessage: error instanceof Error ? error.message : \"Unknown error\",\n\t\tseverity: \"Error\",\n\t\trecoverable: false,\n\t};\n}\n\n",
            );
        }
    }

    generated = generated
        .replace("error: e  as any", "error: __coerceCommandError(e)")
        .replace("error: e as any", "error: __coerceCommandError(e)")
        .replace("// @ts-nocheck\n", "");

    std::fs::write(&out_path, generated).context("write post-processed tauri.ts")?;

    println!("Generated {}", out_path.display());
    Ok(())
}
