extern crate build_support;

fn main() {
    build_support::build_ast_exporter();
    build_support::build_and_translate_test_binaries();
}
