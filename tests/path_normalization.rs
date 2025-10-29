use std::env;

#[test]
fn trims_trailing_spaces_and_strips_quotes() {
    let p = llamapackageservice::utils::normalize_user_input_path("/tmp/OpenResearcher ");
    assert_eq!(p.to_string_lossy(), "/tmp/OpenResearcher");

    let p2 = llamapackageservice::utils::normalize_user_input_path("\"/tmp/OpenResearcher\"");
    assert_eq!(p2.to_string_lossy(), "/tmp/OpenResearcher");

    let p3 = llamapackageservice::utils::normalize_user_input_path("'/tmp/OpenResearcher'");
    assert_eq!(p3.to_string_lossy(), "/tmp/OpenResearcher");
}

#[test]
fn expands_tilde_to_home() {
    let home = env::var("HOME").expect("HOME not set");
    let p = llamapackageservice::utils::normalize_user_input_path("~/Documents");
    assert!(p.starts_with(&home));
}

#[test]
fn url_passthrough_and_local_detection() {
    let url = "https://example.com/repo";
    let out = llamapackageservice::utils::normalize_url_or_path(url);
    assert_eq!(out, url);

    let local = "/var/tmp/foo ";
    let out2 = llamapackageservice::utils::normalize_url_or_path(local);
    assert_eq!(out2, "/var/tmp/foo");
}


