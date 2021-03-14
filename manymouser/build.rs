fn main() {
    let mut build = cc::Build::new();
    build
        .include("manymouse")
        .files(
            [
                "linux_evdev",
                "macosx_hidmanager",
                "macosx_hidutilities",
                "manymouse",
                "windows_wminput",
                "x11_xinput2",
            ]
            .iter()
            .map(|v| format!("manymouse/{}.c", v)),
        )
        .warnings(false)
        .compile("manymouse");
}
