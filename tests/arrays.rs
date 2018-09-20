#[macro_use]
extern crate build_support;

extern crate libc;

use libc::{c_int, c_uint};

import_test_fns! {
    arrays,
    "arrays",
    arrays: {
        fn entry(c_uint, *mut c_int);
    },
    incomplete_arrays: {
        fn test_sized_array() -> c_uint;
        fn entry2(c_uint, *mut c_int);
        fn check_some_ints() -> bool;
    },
    variable_arrays: {
        fn variable_arrays(*mut c_int);
    }
}

mod tests {
    use super::*;

    test_fn!(incomplete_arrays, |test_sized_array| test_sized_array());

    extern "C" {
        #[allow(dead_code)]
        pub static SOME_INTS: [u32; 4];
    }

    test_fn!(incomplete_arrays, |check_some_ints| {
        let res = check_some_ints();
        assert!(res);
        res
    });

    test_fn!(arrays, vec![0; 49], |entry, buffer| {
        let expected_buffer = vec![
            97, 98, 99, 0, 100, 101, 102, 1, 0, 97, 98, 99, 0, 97, 98, 99, 100, 97, 98, 99, 97, 98,
            99, 0, 0, 0, 0, 120, 0, 120, 0, 0, 120, 109, 121, 115, 116, 114, 105, 110, 103, 109,
            121, 115, 116, 114, 105, 110, 103,
        ];

        entry(buffer.len() as u32, buffer.as_mut_ptr());
        assert_eq!(&buffer[..], &expected_buffer[..]);
        buffer
    });

    test_fn!(incomplete_arrays, [0, 0], |entry2, buffer| {
        let expected_buffer = [1, 1];
        entry2(buffer.len() as u32, buffer.as_mut_ptr());
        assert_eq!(expected_buffer, buffer);
        buffer
    });

    test_fn!(variable_arrays, vec![0; 88], |variable_arrays, buffer| {
        let expected_buffer = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 1, 2, 3, 4, 5, 6, 7, 8,
            9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
            31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 0, 3, 6, 9, 12, 15, 18, 21,
        ];
        variable_arrays(buffer.as_mut_ptr());
        assert_eq!(expected_buffer, buffer);
        Vec::from(&buffer[..])
    });
}

#[allow(unused)]
#[no_mangle]
pub static SOME_INTS: [u32; 4] = [2, 0, 1, 8];
