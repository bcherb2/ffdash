use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun if encoder-params.toml changes
    println!("cargo:rerun-if-changed=src/engine/params/encoder-params.toml");

    // Only generate code when dev-tools feature is enabled
    if env::var("CARGO_FEATURE_DEV_TOOLS").is_ok() {
        generate_params_code();
    }
}

/// Validate the TOML registry structure at build time
fn validate_registry(registry: &toml::Value) {
    // Check metadata exists
    let meta = registry.get("meta")
        .and_then(|m| m.as_table())
        .expect("TOML must have [meta] section");

    assert!(meta.get("schema_version").is_some(), "Missing schema_version in [meta]");
    assert!(meta.get("ffmpeg_version").is_some(), "Missing ffmpeg_version in [meta]");
    assert!(meta.get("last_verified").is_some(), "Missing last_verified in [meta]");

    // Check encoders exist and are valid
    let encoders = registry.get("encoders")
        .and_then(|e| e.as_array())
        .expect("TOML must have [[encoders]] array");

    assert!(!encoders.is_empty(), "Must have at least one encoder defined");

    let mut encoder_ids = std::collections::HashSet::new();
    let mut total_params = 0;

    for encoder in encoders {
        let encoder_table = encoder.as_table()
            .expect("Encoder must be a table");

        let id = encoder_table.get("id")
            .and_then(|v| v.as_str())
            .expect("Encoder must have 'id' field");

        // Check for duplicate encoder IDs
        assert!(encoder_ids.insert(id), "Duplicate encoder ID: {}", id);

        // Validate required encoder fields
        assert!(encoder_table.get("codec").is_some(), "Encoder '{}' missing 'codec' field", id);
        assert!(encoder_table.get("type").is_some(), "Encoder '{}' missing 'type' field", id);
        assert!(encoder_table.get("ffmpeg_name").is_some(), "Encoder '{}' missing 'ffmpeg_name' field", id);

        // Validate hardware encoders have hw_api
        if let Some(encoder_type) = encoder_table.get("type").and_then(|v| v.as_str()) {
            if encoder_type == "hardware" {
                assert!(encoder_table.get("hw_api").is_some(),
                    "Hardware encoder '{}' must have 'hw_api' field", id);
            }
        }

        // Validate parameters
        if let Some(params) = encoder_table.get("params").and_then(|p| p.as_array()) {
            let mut param_ids = std::collections::HashSet::new();

            for param in params {
                let param_table = param.as_table()
                    .unwrap_or_else(|| panic!("Param in encoder '{}' must be a table", id));

                let param_id = param_table.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| panic!("Param in encoder '{}' must have 'id' field", id));

                // Check for duplicate param IDs within encoder
                assert!(param_ids.insert(param_id),
                    "Duplicate parameter '{}' in encoder '{}'", param_id, id);

                // Validate required param fields
                assert!(param_table.get("flag").is_some(),
                    "Param '{}' in encoder '{}' missing 'flag' field", param_id, id);
                assert!(param_table.get("type").is_some(),
                    "Param '{}' in encoder '{}' missing 'type' field", param_id, id);
                assert!(param_table.get("description").is_some(),
                    "Param '{}' in encoder '{}' missing 'description' field", param_id, id);

                // Validate flag syntax (must start with -)
                if let Some(flag) = param_table.get("flag").and_then(|v| v.as_str()) {
                    assert!(flag.starts_with('-'),
                        "Flag '{}' for param '{}' in encoder '{}' must start with '-'",
                        flag, param_id, id);
                }

                // Validate ranges
                if let Some(range) = param_table.get("range").and_then(|r| r.as_table()) {
                    if let (Some(min), Some(max)) = (
                        range.get("min").and_then(|v| v.as_integer()),
                        range.get("max").and_then(|v| v.as_integer()),
                    ) {
                        assert!(min <= max,
                            "Param '{}' in encoder '{}' has invalid range: min ({}) > max ({})",
                            param_id, id, min, max);
                    }
                }

                total_params += 1;
            }

            assert!(!param_ids.is_empty(),
                "Encoder '{}' must have at least one parameter defined", id);
        } else {
            panic!("Encoder '{}' has no params defined", id);
        }
    }

    println!("cargo:warning=TOML validation passed: {} encoders, {} total parameter definitions",
        encoder_ids.len(), total_params);
}

#[derive(Debug)]
struct ParamInfo {
    #[allow(dead_code)] // Stored from TOML but not directly accessed - field_path used instead
    id: String,
    scope: String,
    flag: String,
    param_type: String,
    field_path: String,
    range: RangeInfo,
    default: DefaultValue,
    description: String,
    warning: Option<String>,
}

#[derive(Debug)]
enum RangeInfo {
    Int { min: i64, max: i64 },
    Enum { values: Vec<String> },
    Bool,
    Any,
}

#[derive(Debug)]
enum DefaultValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

fn generate_params_code() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("params_generated.rs");

    // Read and parse the TOML file
    let toml_path = "src/engine/params/encoder-params.toml";
    let toml_content = fs::read_to_string(toml_path)
        .expect("Failed to read encoder-params.toml");

    let registry: toml::Value = toml::from_str(&toml_content)
        .expect("Failed to parse encoder-params.toml");

    // Validate TOML structure before code generation
    validate_registry(&registry);

    // Generate the Rust code
    let mut code = String::new();

    // Add imports
    code.push_str("// AUTO-GENERATED by build.rs - DO NOT EDIT MANUALLY\n");
    code.push_str("// Generated from src/engine/params/encoder-params.toml\n\n");
    code.push_str("use crate::engine::params::types::*;\n\n");

    // Extract metadata
    let meta = registry.get("meta").and_then(|m| m.as_table());
    let schema_version = meta
        .and_then(|m| m.get("schema_version"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let ffmpeg_version = meta
        .and_then(|m| m.get("ffmpeg_version"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let last_verified = meta
        .and_then(|m| m.get("last_verified"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Generate metadata constants
    code.push_str(&format!("pub const SCHEMA_VERSION: &str = \"{}\";\n", schema_version));
    code.push_str(&format!("pub const FFMPEG_VERSION: &str = \"{}\";\n", ffmpeg_version));
    code.push_str(&format!("pub const LAST_VERIFIED: &str = \"{}\";\n\n", last_verified));

    // Generate encoders array
    let mut encoder_ids = Vec::new();
    if let Some(encoders) = registry.get("encoders").and_then(|e| e.as_array()) {
        code.push_str("pub static ENCODERS: &[EncoderDef] = &[\n");

        for encoder in encoders {
            let encoder_table = encoder.as_table().expect("Encoder must be a table");

            let id = encoder_table.get("id").and_then(|v| v.as_str()).unwrap();
            encoder_ids.push(id.to_string());

            let codec = encoder_table.get("codec").and_then(|v| v.as_str()).unwrap();
            let encoder_type = encoder_table.get("type").and_then(|v| v.as_str()).unwrap();
            let ffmpeg_name = encoder_table.get("ffmpeg_name").and_then(|v| v.as_str()).unwrap();
            let hw_api = encoder_table.get("hw_api").and_then(|v| v.as_str());

            code.push_str("    EncoderDef {\n");
            code.push_str(&format!("        id: \"{}\",\n", id));
            code.push_str(&format!("        codec: Codec::{},\n",
                if codec == "vp9" { "Vp9" } else { "Av1" }));
            code.push_str(&format!("        encoder_type: EncoderType::{},\n",
                if encoder_type == "software" { "Software" } else { "Hardware" }));
            code.push_str(&format!("        ffmpeg_name: \"{}\",\n", ffmpeg_name));

            if let Some(api) = hw_api {
                let api_variant = match api {
                    "vaapi" => "Vaapi",
                    "qsv" => "Qsv",
                    "nvenc" => "Nvenc",
                    "amf" => "Amf",
                    _ => panic!("Unknown hardware API: {}", api),
                };
                code.push_str(&format!("        hw_api: Some(HardwareApi::{}),\n", api_variant));
            } else {
                code.push_str("        hw_api: None,\n");
            }

            code.push_str("    },\n");
        }

        code.push_str("];\n\n");
    }

    // Collect all parameters across all encoders
    let mut params_map: BTreeMap<String, BTreeMap<String, ParamInfo>> = BTreeMap::new();

    if let Some(encoders) = registry.get("encoders").and_then(|e| e.as_array()) {
        for encoder in encoders {
            let encoder_table = encoder.as_table().expect("Encoder must be a table");
            let encoder_id = encoder_table.get("id").and_then(|v| v.as_str()).unwrap();

            if let Some(params) = encoder_table.get("params").and_then(|p| p.as_array()) {
                for param in params {
                    let param_table = param.as_table().expect("Param must be a table");

                    let param_id = param_table.get("id").and_then(|v| v.as_str()).unwrap();
                    let scope = param_table.get("scope").and_then(|v| v.as_str()).unwrap_or("private");
                    let flag = param_table.get("flag").and_then(|v| v.as_str()).unwrap();
                    let param_type = param_table.get("type").and_then(|v| v.as_str()).unwrap();
                    let description = param_table.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let warning = param_table.get("warning").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let field_path = param_table.get("field_path").and_then(|v| v.as_str())
                        .unwrap_or(&param_id.replace("-", "_")).to_string();

                    // Parse range
                    let range = if let Some(range_table) = param_table.get("range").and_then(|r| r.as_table()) {
                        if let (Some(min), Some(max)) = (
                            range_table.get("min").and_then(|v| v.as_integer()),
                            range_table.get("max").and_then(|v| v.as_integer()),
                        ) {
                            RangeInfo::Int { min, max }
                        } else if let Some(values) = range_table.get("values").and_then(|v| v.as_array()) {
                            let vals: Vec<String> = values.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                            RangeInfo::Enum { values: vals }
                        } else {
                            RangeInfo::Any
                        }
                    } else {
                        match param_type {
                            "bool" => RangeInfo::Bool,
                            _ => RangeInfo::Any,
                        }
                    };

                    // Parse default value
                    let default = if let Some(default_val) = param_table.get("default") {
                        match param_type {
                            "int" => DefaultValue::Int(default_val.as_integer().unwrap_or(0)),
                            "float" => DefaultValue::Float(default_val.as_float().unwrap_or(0.0)),
                            "bool" => DefaultValue::Bool(default_val.as_bool().unwrap_or(false)),
                            "enum" | "string" => DefaultValue::String(default_val.as_str().unwrap_or("").to_string()),
                            _ => DefaultValue::Int(0),
                        }
                    } else {
                        DefaultValue::Int(0)
                    };

                    let param_info = ParamInfo {
                        id: param_id.to_string(),
                        scope: scope.to_string(),
                        flag: flag.to_string(),
                        param_type: param_type.to_string(),
                        field_path: field_path.clone(),
                        range,
                        default,
                        description: description.to_string(),
                        warning,
                    };

                    params_map
                        .entry(param_id.to_string())
                        .or_default()
                        .insert(encoder_id.to_string(), param_info);
                }
            }
        }
    }

    // Generate PARAMS array
    code.push_str("pub static PARAMS: &[ParamDef] = &[\n");

    for (param_id, encoder_params) in params_map.iter() {
        // Get a reference param (use first available)
        let ref_param = encoder_params.values().next().unwrap();

        // Convert param type to Rust type
        let rust_type = match ref_param.param_type.as_str() {
            "int" => match &ref_param.range {
                RangeInfo::Int { min, max } => {
                    if *min >= 0 {
                        if *max <= 255 { "ParamType::U8" }
                        else if *max <= 65535 { "ParamType::U16" }
                        else if *max <= 4294967295 { "ParamType::U32" }
                        else { "ParamType::U64" }
                    } else if *min >= -128 && *max <= 127 { "ParamType::I8" }
                    else if *min >= -32768 && *max <= 32767 { "ParamType::I16" }
                    else if *min >= -2147483648 && *max <= 2147483647 { "ParamType::I32" }
                    else { "ParamType::I64" }
                }
                _ => "ParamType::I32",
            },
            "float" => "ParamType::F32",
            "bool" => "ParamType::Bool",
            "enum" | "string" => "ParamType::String",
            _ => "ParamType::String",
        };

        // Generate range
        let range_code = match &ref_param.range {
            RangeInfo::Int { min, max } => format!("Range::Int {{ min: {}, max: {} }}", min, max),
            RangeInfo::Enum { values } => {
                let vals_str = values.iter()
                    .map(|v| format!("\"{}\".to_string()", v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Range::Enum {{ values: vec![{}] }}", vals_str)
            }
            RangeInfo::Bool => "Range::Bool".to_string(),
            RangeInfo::Any => "Range::Any".to_string(),
        };

        // Generate default value
        let default_code = match &ref_param.default {
            DefaultValue::Int(v) => format!("Value::I32({} as i32)", v),
            DefaultValue::Float(v) => format!("Value::F32({} as f32)", v),
            DefaultValue::Bool(v) => format!("Value::Bool({})", v),
            DefaultValue::String(v) => format!("Value::Str(\"{}\")", v),
        };

        code.push_str("    ParamDef {\n");
        code.push_str(&format!("        name: \"{}\",\n", param_id));
        code.push_str(&format!("        field_path: \"{}\",\n", ref_param.field_path));
        code.push_str(&format!("        rust_type: {},\n", rust_type));
        code.push_str(&format!("        description: \"{}\",\n",
            ref_param.description.replace("\"", "\\\"")));
        code.push_str(&format!("        group: \"{}\",\n", ref_param.scope));
        code.push_str(&format!("        range: {},\n", range_code));
        code.push_str(&format!("        default: {},\n", default_code));
        code.push_str("        encoder_support: &[\n");

        // Generate encoder support for all encoders
        for encoder_id in &encoder_ids {
            if let Some(param_info) = encoder_params.get(encoder_id) {
                // Check if this encoder has a different range than the reference
                let range_override = match (&ref_param.range, &param_info.range) {
                    (RangeInfo::Int { min: ref_min, max: ref_max },
                     RangeInfo::Int { min: enc_min, max: enc_max })
                        if ref_min != enc_min || ref_max != enc_max => {
                        format!("Some(Range::Int {{ min: {}, max: {} }})", enc_min, enc_max)
                    },
                    (RangeInfo::Enum { values: ref_vals },
                     RangeInfo::Enum { values: enc_vals })
                        if ref_vals != enc_vals => {
                        let vals_str = enc_vals.iter()
                            .map(|v| format!("\"{}\".to_string()", v))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("Some(Range::Enum {{ values: vec![{}] }})", vals_str)
                    },
                    _ => "None".to_string(),
                };

                code.push_str(&format!("            (\"{}\", EncoderParam {{\n", encoder_id));
                code.push_str(&format!("                flag: Some(\"{}\"),\n", param_info.flag));
                code.push_str("                supported: true,\n");
                code.push_str("                reason: None,\n");
                code.push_str(&format!("                range_override: {},\n", range_override));
                code.push_str("                condition: Condition::Always,\n");

                if let Some(warning) = &param_info.warning {
                    code.push_str(&format!("                note: Some(\"{}\"),\n",
                        warning.replace("\"", "\\\"")));
                } else {
                    code.push_str("                note: None,\n");
                }

                code.push_str("            }),\n");
            } else {
                // Encoder doesn't support this param
                code.push_str(&format!("            (\"{}\", EncoderParam {{\n", encoder_id));
                code.push_str("                flag: None,\n");
                code.push_str("                supported: false,\n");
                code.push_str("                reason: Some(\"Not supported by this encoder\"),\n");
                code.push_str("                range_override: None,\n");
                code.push_str("                condition: Condition::Always,\n");
                code.push_str("                note: None,\n");
                code.push_str("            }),\n");
            }
        }

        code.push_str("        ],\n");
        code.push_str("    },\n");
    }

    code.push_str("];\n");

    // Write the generated code
    fs::write(&dest_path, code)
        .expect("Failed to write generated code");

    println!("Generated params code at: {}", dest_path.display());
    println!("Generated {} encoders and {} parameters", encoder_ids.len(), params_map.len());
}
