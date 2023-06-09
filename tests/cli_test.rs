#[test]
fn cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/symlink_flag.trycmd");
    trycmd::TestCases::new().case("tests/cmd/*.toml");
}
