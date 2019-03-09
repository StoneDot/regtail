/*
 * Copyright 2019 StoneDot (Hiroaki Goto)
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

macro_rules! test {
    ($test_name:ident, $test_code:expr) => {
        #[test]
        fn $test_name() {
            let (dir, command) = utils::setup(stringify!($test_name));
            $test_code(dir, command);
        }
    };
}

#[allow(unused_macros)]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {{
        match (&$haystack, &$needle) {
            (haystack, needle) => {
                if !haystack.contains(needle) {
                    panic!(
                        r#"assertion failed: `(haystack contains needle)`
   needle: `{:?}`,
 haystack: `{:?}`"#,
                        needle, haystack
                    )
                }
            }
        }
    }};
    ($haystack:expr, $needle:expr,) => {{
        assert_contains!($haystack, $needle)
    }};
}

#[allow(unused_macros)]
macro_rules! assert_not_contains {
    ($haystack:expr, $needle:expr) => {{
        match (&$haystack, &$needle) {
            (haystack, needle) => {
                if haystack.contains(needle) {
                    panic!(
                        r#"assertion failed: `(haystack not contains needle)`
   needle: `{:?}`,
 haystack: `{:?}`"#,
                        needle, haystack
                    )
                }
            }
        }
    }}
}
