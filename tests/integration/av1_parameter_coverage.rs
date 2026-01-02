use ffdash::engine::Profile;

use crate::common::assertions::assert_cmd_contains;
use crate::common::helpers::{
    build_av1_nvenc_cmd_for_test, build_av1_qsv_cmd_for_test, build_av1_software_cmd_for_test,
    build_av1_vaapi_cmd_for_test,
};
use crate::common::parameter_registry_av1::get_av1_parameter_mappings;

#[test]
fn test_av1_parameter_registry_coverage() {
    let mappings = get_av1_parameter_mappings();

    let svt_cmd = build_av1_software_cmd_for_test(&Profile::get("av1-svt"));
    let qsv_cmd = build_av1_qsv_cmd_for_test(&Profile::get("av1-qsv"));
    let nvenc_cmd = build_av1_nvenc_cmd_for_test(&Profile::get("av1-nvenc"));
    let vaapi_cmd = build_av1_vaapi_cmd_for_test(&Profile::get("av1-vaapi"));

    for mapping in mappings {
        if let Some(flag) = mapping.svt_flag {
            assert_cmd_contains(&svt_cmd, flag);
        }
        if let Some(flag) = mapping.qsv_flag {
            assert_cmd_contains(&qsv_cmd, flag);
        }
        if let Some(flag) = mapping.nvenc_flag {
            assert_cmd_contains(&nvenc_cmd, flag);
        }
        if let Some(flag) = mapping.vaapi_flag {
            assert_cmd_contains(&vaapi_cmd, flag);
        }
    }
}
