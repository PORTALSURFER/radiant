use super::*;

#[test]
fn native_external_drag_dropfiles_payload_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let payload = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/external_drag/payload.rs"),
    )
    .expect("native external drag payload module should be readable");
    let dropfiles =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/external_drag/payload/dropfiles.rs",
        ))
        .expect("native external drag DROPFILES payload module should be readable");

    assert!(
        payload.contains("mod dropfiles;")
            && payload.contains("use dropfiles::build_dropfiles_payload;"),
        "external drag payload module should delegate CF_HDROP path serialization"
    );
    assert!(
        !payload.contains("fn encode_drag_paths")
            && !payload.contains("fn dropfiles_header_bytes")
            && !payload.contains("DROPFILES")
            && dropfiles.contains("fn encode_drag_paths")
            && dropfiles.contains("fn dropfiles_header_bytes")
            && dropfiles.contains("DROPFILES"),
        "DROPFILES header and UTF-16 path serialization should live in payload/dropfiles.rs"
    );
}

#[test]
fn native_external_drag_data_object_helpers_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_object = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/external_drag/data_object.rs"),
    )
    .expect("native external drag data object module should be readable");
    let formats =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/formats.rs",
        ))
        .expect("native external drag data object format helper should be readable");
    let medium =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/medium.rs",
        ))
        .expect("native external drag data object medium helper should be readable");

    assert!(
        data_object.contains("mod formats;")
            && data_object.contains("mod medium;")
            && data_object.contains("data_object_format_matches")
            && data_object.contains("drop_effect_from_medium")
            && !data_object.contains("fn is_file_drop_format")
            && !data_object.contains("GlobalLock"),
        "external drag IDataObject implementation should delegate format matching and HGLOBAL effect decoding"
    );
    assert!(
        formats.contains("fn data_object_format_matches")
            && formats.contains("fn is_file_drop_format")
            && formats.contains("fn is_drop_effect_format")
            && formats.contains("fn uses_hglobal_storage")
            && medium.contains("fn drop_effect_from_medium")
            && medium.contains("GlobalLock")
            && medium.contains("GlobalUnlock"),
        "external drag data-object helpers should stay grouped by FORMATETC matching and STGMEDIUM decoding"
    );
}
