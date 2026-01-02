/// Minimal AV1 parameter registry for parity tests across encoders

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Av1ParameterMapping {
    pub field_name: &'static str,
    pub svt_flag: Option<&'static str>,
    pub qsv_flag: Option<&'static str>,
    pub nvenc_flag: Option<&'static str>,
    pub vaapi_flag: Option<&'static str>,
}

#[allow(dead_code)]
pub fn get_av1_parameter_mappings() -> Vec<Av1ParameterMapping> {
    vec![
        Av1ParameterMapping {
            field_name: "video_codec",
            svt_flag: Some("-c:v libsvtav1"),
            qsv_flag: Some("-c:v av1_qsv"),
            nvenc_flag: Some("-c:v av1_nvenc"),
            vaapi_flag: Some("-c:v av1_vaapi"),
        },
        Av1ParameterMapping {
            field_name: "quality",
            svt_flag: Some("-crf"),
            qsv_flag: Some("-q:v"),
            nvenc_flag: Some("-cq"),
            vaapi_flag: Some("-global_quality"),
        },
        Av1ParameterMapping {
            field_name: "preset",
            svt_flag: Some("-preset"),
            qsv_flag: Some("-preset"),
            nvenc_flag: Some("-preset"),
            vaapi_flag: None,
        },
        Av1ParameterMapping {
            field_name: "svt_params",
            svt_flag: Some("-svtav1-params"),
            qsv_flag: None,
            nvenc_flag: None,
            vaapi_flag: None,
        },
        Av1ParameterMapping {
            field_name: "gop",
            svt_flag: Some("-g:v"),
            qsv_flag: Some("-g:v"),
            nvenc_flag: Some("-g:v"),
            vaapi_flag: Some("-g:v"),
        },
        Av1ParameterMapping {
            field_name: "pix_fmt",
            svt_flag: Some("-pix_fmt"),
            qsv_flag: Some("-pix_fmt"),
            nvenc_flag: Some("-pix_fmt"),
            vaapi_flag: Some("format=nv12"),
        },
        Av1ParameterMapping {
            field_name: "hw_init",
            svt_flag: None,
            qsv_flag: Some("-init_hw_device qsv"),
            nvenc_flag: Some("-hwaccel cuda"),
            vaapi_flag: Some("-init_hw_device vaapi"),
        },
        Av1ParameterMapping {
            field_name: "lookahead",
            svt_flag: None,
            qsv_flag: Some("-look_ahead"),
            nvenc_flag: Some("-rc-lookahead"),
            vaapi_flag: None,
        },
        Av1ParameterMapping {
            field_name: "b_frames",
            svt_flag: None,
            qsv_flag: Some("-bf"),
            nvenc_flag: None,
            vaapi_flag: None,
        },
        Av1ParameterMapping {
            field_name: "hwupload",
            svt_flag: None,
            qsv_flag: Some("vpp_qsv"),
            nvenc_flag: None,
            vaapi_flag: Some("hwupload"),
        },
        Av1ParameterMapping {
            field_name: "audio_codec",
            svt_flag: Some("-c:a"),
            qsv_flag: Some("-c:a"),
            nvenc_flag: Some("-c:a"),
            vaapi_flag: Some("-c:a"),
        },
        Av1ParameterMapping {
            field_name: "audio_bitrate",
            svt_flag: Some("-b:a"),
            qsv_flag: Some("-b:a"),
            nvenc_flag: Some("-b:a"),
            vaapi_flag: Some("-b:a"),
        },
    ]
}
