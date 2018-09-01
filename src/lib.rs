#![feature(rustc_private)]
extern crate dtoa;
extern crate indexmap;
extern crate rustc_target;
extern crate serde;
extern crate serde_cbor;
extern crate serde_json;
extern crate syntax;
extern crate syntax_pos;

pub mod c_ast;
pub mod cfg;
pub mod clang_ast;
pub mod convert_type;
pub mod loops;
pub mod renamer;
pub mod rust_ast;
pub mod translator;
pub mod with_stmts;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
