#[macro_use]
extern crate serde_json;

extern crate cc;
extern crate globwalk;
extern crate hostname;
extern crate libc;
extern crate regex;
extern crate syn;

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    process::Command,
};

// A directory test goes through the following set of steps:
//
// 1. A `compile_commands.json` file is created for the Clang plugin in `ast-exporter` to recognize
// its C source input
// 2. This JSON and the C source file are fed to the `ast-exporter` to produce a CBOR file of the
// Clang type-annotated abstract syntax tree.
// 3. This CBOR file is fed to the `ast-importer` to produce a Rust source file supposedly
// preserving the semantics of the initial C source file.
// 4. Rust test files (test_xyz.rs) are compiled into a single main wrapper and main test binary
// and are automatically linked against other Rust and C files thanks to `rustc`.
// 5. The executable from the previous step is run one or more times parameterized to a specific
// test function.

#[macro_export]
macro_rules! import_test_fns {
    (
        $dir:ident,
        $lname:expr,
        $($module:ident: {
            $(
                fn $test_fn:ident($($argty:ty),*) $(-> $retty:ty)*;
            )*
        }),*
    ) => {
        mod c {
            use super::*;

            #[link(name = $lname)]
            extern "C" {
                $($(
                    #[no_mangle]
                    pub fn $test_fn( $(_: $argty),* ) $(-> $retty)*;
                )*)*
            }
        }

        // FIXME(anp): remove this stub module and actually do the translation!
        // $( use $module; )*
        $(
            mod $module {
                use super::*;
                $(
                    pub fn $test_fn( $(_: $argty),* )  $(-> $retty)* {
                        unimplemented!();
                    }
                )*
            }
        )*
    };
}

#[macro_export]
macro_rules! test_fn {
    (
        $module:ident,
        $init:expr,
        |$fn:ident, $initd:ident| $f:expr
    ) => {
        #[test]
        fn $fn() {
            #![allow(unused_mut, unused_unsafe)]
            let original = unsafe {
                use self::c::$fn;
                let mut $initd = (|| $init)();
                $f
            };

            let converted = unsafe {
                use self::$module::$fn;
                let mut $initd = (|| $init)();
                $f
            };

            assert_eq!(converted, original);
        }
    };
    (
        $module:ident,
        |$fn:ident| $f:expr
    ) => {
        test_fn!($module, {}, |$fn, _unit| $f);
    };
}

pub fn build_ast_exporter() {
    unimplemented!();
}

pub fn build_and_translate_test_binaries() {
    let test_dir_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests");

    eprintln!("reading test directory at {}", test_dir_path.display());
    for d in read_dir(&test_dir_path).unwrap() {
        let path = d.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        build_and_translate_single_test(path);
    }
}

fn build_and_translate_single_test(path: impl AsRef<Path>) {
    let path = path.as_ref();
    eprintln!("building and translating test dir @ {}", path.display());
    let mut build = cc::Build::new();

    read_dir(&path)
        .unwrap()
        .map(|d| d.unwrap().path().to_owned())
        .filter(|p| p.extension().map(|e| e == "c") == Some(true))
        .for_each(|f| {
            // set the c code up for compilation
            build.file(&f);
            let cbor_path = export_cbor_file(&f);
            let translated = translate(&f, &cbor_path);
            // FIXME(anp): write the translated file to disk
        });

    build
        .pic(true)
        .static_flag(true)
        .warnings(false)
        .extra_warnings(false)
        .cargo_metadata(false);

    let libname = path.file_name().unwrap().to_string_lossy().to_owned();

    eprintln!("compiling {}", libname);
    build.compile(&libname);

    println!(
        "cargo:rustc-link-search=native={}",
        ::std::env::var("OUT_DIR").unwrap()
    );
}

fn export_cbor_file(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    eprintln!("exporting cbor file for {}", path.display());
    let test_dir = path.parent().unwrap().file_name().unwrap();

    let directory = path.parent().unwrap();
    let cfile = path.file_name().unwrap();
    let cfile_pretty = Path::new(cfile).display().to_string();

    let compile_commands = json! {[
        {
            "arguments": [ "cc", "-D_FORTIFY_SOURCE=0", "-c", cfile_pretty ],
            "directory": directory,
            "file": cfile_pretty,
        }
    ]};

    let compile_commands = serde_json::to_string_pretty(&compile_commands).unwrap();
    let compile_commands_path = PathBuf::from(::std::env::var("OUT_DIR").unwrap())
        .join(&test_dir)
        .join("compile-commands.json");
    eprintln!(
        "writing compile commands to {}",
        compile_commands_path.display()
    );

    eprintln!("writing compile commands to {}", compile_commands_path.display());
    ::std::fs::create_dir_all(compile_commands_path.parent().unwrap()).unwrap();
    ::std::fs::write(&compile_commands_path, &compile_commands).unwrap();

    let ast_exporter_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("dependencies")
        //  FIXME(anp)  i'm hacking at this quickly, sue me
        .join("llvm-6.0.1")
        .join(format!("build.{}", hostname::get_hostname().unwrap()))
        .join("bin")
        .join("ast-exporter");

    eprintln!("running ast exported from {}", ast_exporter_path.display());
    Command::new(ast_exporter_path)
        .arg("-p") // tell the clang frontend where to get the compile_commands.json file
        .arg(&compile_commands_path.parent().unwrap())
        .arg(&path)
        .status()
        .unwrap();

    PathBuf::from(format!("{}.cbor", path.display()))
}

fn translate(c_path: impl AsRef<Path>, cbor_path: impl AsRef<Path>) -> String {
    // c_file_path, _ = os.path.splitext(self.path)
    // extensionless_file, _ = os.path.splitext(c_file_path)
    // rust_src = extensionless_file + ".rs"

    // # help plumbum find rust
    // ld_lib_path = get_rust_toolchain_libpath()
    // if 'LD_LIBRARY_PATH' in pb.local.env:
    //     ld_lib_path += ':' + pb.local.env['LD_LIBRARY_PATH']

    // # run the importer
    // ast_importer = get_cmd_or_die(c.AST_IMPO)

    // args = [
    //     self.path,
    // ]

    // if self.enable_relooper:
    //     args.append("--reloop-cfgs")
    //     #  args.append("--use-c-loop-info")
    //     #  args.append("--use-c-multiple-info")
    // if self.disallow_current_block:
    //     args.append("--fail-on-multiple")

    // with pb.local.env(RUST_BACKTRACE='1', LD_LIBRARY_PATH=ld_lib_path):
    //     # log the command in a format that's easy to re-run
    //     translation_cmd = "LD_LIBRARY_PATH=" + ld_lib_path + " \\\n"
    //     translation_cmd += str(ast_importer[args] > rust_src)
    //     logging.debug("translation command:\n %s", translation_cmd)
    //     retcode, stdout, stderr = (ast_importer[args] > rust_src).run(
    //         retcode=None)

    // logging.debug("stdout:\n%s", stdout)

    // if retcode != 0:
    //     raise NonZeroReturn(stderr)

    // return RustFile(extensionless_file + ".rs")
    unimplemented!();
}

// rust_file_builder = RustFileBuilder()
// rust_file_builder.add_features(["libc", "extern_types", "used"])

// rust_file_builder.add_mod(RustMod(extensionless_rust_file,
//                                   RustVisibility.Public))
// }
