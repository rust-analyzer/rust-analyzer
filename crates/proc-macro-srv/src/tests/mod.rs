//! proc-macro tests

#[macro_use]
mod utils;
use utils::*;

use expect_test::expect;

#[test]
fn test_derive_empty() {
    assert_expand(
        "DeriveEmpty",
        r#"struct S { field: &'r#lt fn(u32) -> &'a r#u32 }"#,
        expect![[r#"
            IDENT   struct 1
            IDENT   S 1
            GROUP {} 1 1
              IDENT   field 1
              PUNCT   : [alone] 1
              PUNCT   & [joint] 1
              PUNCT   ' [joint] 1
              IDENT   r#lt 1
              IDENT   fn 1
              GROUP () 1 1
                IDENT   u32 1
              PUNCT   - [joint] 1
              PUNCT   > [alone] 1
              PUNCT   & [joint] 1
              PUNCT   ' [joint] 1
              IDENT   a 1
              IDENT   r#u32 1
        "#]],
        expect![[r#"
            IDENT   struct 42:Root[0000, 0]@0..6#1
            IDENT   S 42:Root[0000, 0]@7..8#1
            GROUP {} 42:Root[0000, 0]@9..10#1 42:Root[0000, 0]@46..47#1
              IDENT   field 42:Root[0000, 0]@11..16#1
              PUNCT   : [alone] 42:Root[0000, 0]@16..17#1
              PUNCT   & [joint] 42:Root[0000, 0]@18..19#1
              PUNCT   ' [joint] 42:Root[0000, 0]@22..23#1
              IDENT   r#lt 42:Root[0000, 0]@22..24#1
              IDENT   fn 42:Root[0000, 0]@25..27#1
              GROUP () 42:Root[0000, 0]@27..28#1 42:Root[0000, 0]@31..32#1
                IDENT   u32 42:Root[0000, 0]@28..31#1
              PUNCT   - [joint] 42:Root[0000, 0]@33..34#1
              PUNCT   > [alone] 42:Root[0000, 0]@34..35#1
              PUNCT   & [joint] 42:Root[0000, 0]@36..37#1
              PUNCT   ' [joint] 42:Root[0000, 0]@38..39#1
              IDENT   a 42:Root[0000, 0]@38..39#1
              IDENT   r#u32 42:Root[0000, 0]@42..45#1
        "#]],
    );
}

#[test]
fn test_derive_reemit_helpers() {
    assert_expand(
        "DeriveReemit",
        r#"
#[helper(build_fn(private, name = "partial_build"))]
pub struct Foo {
    /// The domain where this federated instance is running
    #[helper(setter(into))]
    pub(crate) domain: String,
}
"#,
        expect![[r#"
            PUNCT   # [joint] 1
            GROUP [] 1 1
              IDENT   helper 1
              GROUP () 1 1
                IDENT   build_fn 1
                GROUP () 1 1
                  IDENT   private 1
                  PUNCT   , [alone] 1
                  IDENT   name 1
                  PUNCT   = [alone] 1
                  LITERAL Str partial_build 1
            IDENT   pub 1
            IDENT   struct 1
            IDENT   Foo 1
            GROUP {} 1 1
              PUNCT   # [alone] 1
              GROUP [] 1 1
                IDENT   doc 1
                PUNCT   = [alone] 1
                LITERAL Str  The domain where this federated instance is running 1
              PUNCT   # [joint] 1
              GROUP [] 1 1
                IDENT   helper 1
                GROUP () 1 1
                  IDENT   setter 1
                  GROUP () 1 1
                    IDENT   into 1
              IDENT   pub 1
              GROUP () 1 1
                IDENT   crate 1
              IDENT   domain 1
              PUNCT   : [alone] 1
              IDENT   String 1
              PUNCT   , [alone] 1


            PUNCT   # [joint] 1
            GROUP [] 1 1
              IDENT   helper 1
              GROUP () 1 1
                IDENT   build_fn 1
                GROUP () 1 1
                  IDENT   private 1
                  PUNCT   , [alone] 1
                  IDENT   name 1
                  PUNCT   = [alone] 1
                  LITERAL Str partial_build 1
            IDENT   pub 1
            IDENT   struct 1
            IDENT   Foo 1
            GROUP {} 1 1
              PUNCT   # [alone] 1
              GROUP [] 1 1
                IDENT   doc 1
                PUNCT   = [alone] 1
                LITERAL Str  The domain where this federated instance is running 1
              PUNCT   # [joint] 1
              GROUP [] 1 1
                IDENT   helper 1
                GROUP () 1 1
                  IDENT   setter 1
                  GROUP () 1 1
                    IDENT   into 1
              IDENT   pub 1
              GROUP () 1 1
                IDENT   crate 1
              IDENT   domain 1
              PUNCT   : [alone] 1
              IDENT   String 1
              PUNCT   , [alone] 1
        "#]],
        expect![[r#"
            PUNCT   # [joint] 42:Root[0000, 0]@1..2#1
            GROUP [] 42:Root[0000, 0]@2..3#1 42:Root[0000, 0]@52..53#1
              IDENT   helper 42:Root[0000, 0]@3..9#1
              GROUP () 42:Root[0000, 0]@9..10#1 42:Root[0000, 0]@51..52#1
                IDENT   build_fn 42:Root[0000, 0]@10..18#1
                GROUP () 42:Root[0000, 0]@18..19#1 42:Root[0000, 0]@50..51#1
                  IDENT   private 42:Root[0000, 0]@19..26#1
                  PUNCT   , [alone] 42:Root[0000, 0]@26..27#1
                  IDENT   name 42:Root[0000, 0]@28..32#1
                  PUNCT   = [alone] 42:Root[0000, 0]@33..34#1
                  LITERAL Str partial_build 42:Root[0000, 0]@35..50#1
            IDENT   pub 42:Root[0000, 0]@54..57#1
            IDENT   struct 42:Root[0000, 0]@58..64#1
            IDENT   Foo 42:Root[0000, 0]@65..68#1
            GROUP {} 42:Root[0000, 0]@69..70#1 42:Root[0000, 0]@190..191#1
              PUNCT   # [alone] 42:Root[0000, 0]@0..0#1
              GROUP [] 42:Root[0000, 0]@75..130#1 42:Root[0000, 0]@75..130#1
                IDENT   doc 42:Root[0000, 0]@75..130#1
                PUNCT   = [alone] 42:Root[0000, 0]@75..130#1
                LITERAL Str  The domain where this federated instance is running 42:Root[0000, 0]@75..130#1
              PUNCT   # [joint] 42:Root[0000, 0]@135..136#1
              GROUP [] 42:Root[0000, 0]@136..137#1 42:Root[0000, 0]@157..158#1
                IDENT   helper 42:Root[0000, 0]@137..143#1
                GROUP () 42:Root[0000, 0]@143..144#1 42:Root[0000, 0]@156..157#1
                  IDENT   setter 42:Root[0000, 0]@144..150#1
                  GROUP () 42:Root[0000, 0]@150..151#1 42:Root[0000, 0]@155..156#1
                    IDENT   into 42:Root[0000, 0]@151..155#1
              IDENT   pub 42:Root[0000, 0]@163..166#1
              GROUP () 42:Root[0000, 0]@166..167#1 42:Root[0000, 0]@172..173#1
                IDENT   crate 42:Root[0000, 0]@167..172#1
              IDENT   domain 42:Root[0000, 0]@174..180#1
              PUNCT   : [alone] 42:Root[0000, 0]@180..181#1
              IDENT   String 42:Root[0000, 0]@182..188#1
              PUNCT   , [alone] 42:Root[0000, 0]@188..189#1


            PUNCT   # [joint] 42:Root[0000, 0]@1..2#1
            GROUP [] 42:Root[0000, 0]@2..3#1 42:Root[0000, 0]@52..53#1
              IDENT   helper 42:Root[0000, 0]@3..9#1
              GROUP () 42:Root[0000, 0]@9..10#1 42:Root[0000, 0]@51..52#1
                IDENT   build_fn 42:Root[0000, 0]@10..18#1
                GROUP () 42:Root[0000, 0]@18..19#1 42:Root[0000, 0]@50..51#1
                  IDENT   private 42:Root[0000, 0]@19..26#1
                  PUNCT   , [alone] 42:Root[0000, 0]@26..27#1
                  IDENT   name 42:Root[0000, 0]@28..32#1
                  PUNCT   = [alone] 42:Root[0000, 0]@33..34#1
                  LITERAL Str partial_build 42:Root[0000, 0]@35..50#1
            IDENT   pub 42:Root[0000, 0]@54..57#1
            IDENT   struct 42:Root[0000, 0]@58..64#1
            IDENT   Foo 42:Root[0000, 0]@65..68#1
            GROUP {} 42:Root[0000, 0]@69..70#1 42:Root[0000, 0]@190..191#1
              PUNCT   # [alone] 42:Root[0000, 0]@0..0#1
              GROUP [] 42:Root[0000, 0]@75..130#1 42:Root[0000, 0]@75..130#1
                IDENT   doc 42:Root[0000, 0]@75..130#1
                PUNCT   = [alone] 42:Root[0000, 0]@75..130#1
                LITERAL Str  The domain where this federated instance is running 42:Root[0000, 0]@75..130#1
              PUNCT   # [joint] 42:Root[0000, 0]@135..136#1
              GROUP [] 42:Root[0000, 0]@136..137#1 42:Root[0000, 0]@157..158#1
                IDENT   helper 42:Root[0000, 0]@137..143#1
                GROUP () 42:Root[0000, 0]@143..144#1 42:Root[0000, 0]@156..157#1
                  IDENT   setter 42:Root[0000, 0]@144..150#1
                  GROUP () 42:Root[0000, 0]@150..151#1 42:Root[0000, 0]@155..156#1
                    IDENT   into 42:Root[0000, 0]@151..155#1
              IDENT   pub 42:Root[0000, 0]@163..166#1
              GROUP () 42:Root[0000, 0]@166..167#1 42:Root[0000, 0]@172..173#1
                IDENT   crate 42:Root[0000, 0]@167..172#1
              IDENT   domain 42:Root[0000, 0]@174..180#1
              PUNCT   : [alone] 42:Root[0000, 0]@180..181#1
              IDENT   String 42:Root[0000, 0]@182..188#1
              PUNCT   , [alone] 42:Root[0000, 0]@188..189#1
        "#]],
    );
}

#[test]
fn test_derive_error() {
    assert_expand(
        "DeriveError",
        r#"struct S { field: u32 }"#,
        expect![[r#"
            IDENT   struct 1
            IDENT   S 1
            GROUP {} 1 1
              IDENT   field 1
              PUNCT   : [alone] 1
              IDENT   u32 1


            IDENT   compile_error 1
            PUNCT   ! [joint] 1
            GROUP () 1 1
              LITERAL Str #[derive(DeriveError)] struct S {field : u32} 1
            PUNCT   ; [alone] 1
        "#]],
        expect![[r#"
            IDENT   struct 42:Root[0000, 0]@0..6#1
            IDENT   S 42:Root[0000, 0]@7..8#1
            GROUP {} 42:Root[0000, 0]@9..10#1 42:Root[0000, 0]@22..23#1
              IDENT   field 42:Root[0000, 0]@11..16#1
              PUNCT   : [alone] 42:Root[0000, 0]@16..17#1
              IDENT   u32 42:Root[0000, 0]@18..21#1


            IDENT   compile_error 42:Root[0000, 0]@0..13#1
            PUNCT   ! [joint] 42:Root[0000, 0]@13..14#1
            GROUP () 42:Root[0000, 0]@14..15#1 42:Root[0000, 0]@62..63#1
              LITERAL Str #[derive(DeriveError)] struct S {field : u32} 42:Root[0000, 0]@15..62#1
            PUNCT   ; [alone] 42:Root[0000, 0]@63..64#1
        "#]],
    );
}

#[test]
fn test_fn_like_macro_noop() {
    assert_expand(
        "fn_like_noop",
        r#"ident, 0, 1, []"#,
        expect![[r#"
            IDENT   ident 1
            PUNCT   , [alone] 1
            LITERAL Integer 0 1
            PUNCT   , [alone] 1
            LITERAL Integer 1 1
            PUNCT   , [alone] 1
            GROUP [] 1 1


            IDENT   ident 1
            PUNCT   , [alone] 1
            LITERAL Integer 0 1
            PUNCT   , [alone] 1
            LITERAL Integer 1 1
            PUNCT   , [alone] 1
            GROUP [] 1 1
        "#]],
        expect![[r#"
            IDENT   ident 42:Root[0000, 0]@0..5#1
            PUNCT   , [alone] 42:Root[0000, 0]@5..6#1
            LITERAL Integer 0 42:Root[0000, 0]@7..8#1
            PUNCT   , [alone] 42:Root[0000, 0]@8..9#1
            LITERAL Integer 1 42:Root[0000, 0]@10..11#1
            PUNCT   , [alone] 42:Root[0000, 0]@11..12#1
            GROUP [] 42:Root[0000, 0]@13..14#1 42:Root[0000, 0]@14..15#1


            IDENT   ident 42:Root[0000, 0]@0..5#1
            PUNCT   , [alone] 42:Root[0000, 0]@5..6#1
            LITERAL Integer 0 42:Root[0000, 0]@7..8#1
            PUNCT   , [alone] 42:Root[0000, 0]@8..9#1
            LITERAL Integer 1 42:Root[0000, 0]@10..11#1
            PUNCT   , [alone] 42:Root[0000, 0]@11..12#1
            GROUP [] 42:Root[0000, 0]@13..14#1 42:Root[0000, 0]@14..15#1
        "#]],
    );
}

#[test]
fn test_fn_like_macro_clone_ident_subtree() {
    assert_expand(
        "fn_like_clone_tokens",
        r#"ident, [ident2, ident3]"#,
        expect![[r#"
            IDENT   ident 1
            PUNCT   , [alone] 1
            GROUP [] 1 1
              IDENT   ident2 1
              PUNCT   , [alone] 1
              IDENT   ident3 1


            IDENT   ident 1
            PUNCT   , [alone] 1
            GROUP [] 1 1
              IDENT   ident2 1
              PUNCT   , [alone] 1
              IDENT   ident3 1
        "#]],
        expect![[r#"
            IDENT   ident 42:Root[0000, 0]@0..5#1
            PUNCT   , [alone] 42:Root[0000, 0]@5..6#1
            GROUP [] 42:Root[0000, 0]@7..8#1 42:Root[0000, 0]@22..23#1
              IDENT   ident2 42:Root[0000, 0]@8..14#1
              PUNCT   , [alone] 42:Root[0000, 0]@14..15#1
              IDENT   ident3 42:Root[0000, 0]@16..22#1


            IDENT   ident 42:Root[0000, 0]@0..5#1
            PUNCT   , [alone] 42:Root[0000, 0]@5..6#1
            GROUP [] 42:Root[0000, 0]@7..23#1 42:Root[0000, 0]@7..23#1
              IDENT   ident2 42:Root[0000, 0]@8..14#1
              PUNCT   , [alone] 42:Root[0000, 0]@14..15#1
              IDENT   ident3 42:Root[0000, 0]@16..22#1
        "#]],
    );
}

#[test]
fn test_fn_like_macro_clone_raw_ident() {
    assert_expand(
        "fn_like_clone_tokens",
        "r#async",
        expect![[r#"
            IDENT   r#async 1


            IDENT   r#async 1
        "#]],
        expect![[r#"
            IDENT   r#async 42:Root[0000, 0]@2..7#1


            IDENT   r#async 42:Root[0000, 0]@2..7#1
        "#]],
    );
}

#[test]
fn test_fn_like_fn_like_span_join() {
    assert_expand(
        "fn_like_span_join",
        "foo     bar",
        expect![[r#"
            IDENT   foo 1
            IDENT   bar 1


            IDENT   r#joined 1
        "#]],
        expect![[r#"
            IDENT   foo 42:Root[0000, 0]@0..3#1
            IDENT   bar 42:Root[0000, 0]@8..11#1


            IDENT   r#joined 42:Root[0000, 0]@0..11#1
        "#]],
    );
}

#[test]
fn test_fn_like_fn_like_span_ops() {
    assert_expand(
        "fn_like_span_ops",
        "set_def_site resolved_at_def_site start_span",
        expect![[r#"
            IDENT   set_def_site 1
            IDENT   resolved_at_def_site 1
            IDENT   start_span 1


            IDENT   set_def_site 0
            IDENT   resolved_at_def_site 1
            IDENT   start_span 1
        "#]],
        expect![[r#"
            IDENT   set_def_site 42:Root[0000, 0]@0..12#1
            IDENT   resolved_at_def_site 42:Root[0000, 0]@13..33#1
            IDENT   start_span 42:Root[0000, 0]@34..44#1


            IDENT   set_def_site 41:Root[0000, 0]@0..150#1
            IDENT   resolved_at_def_site 42:Root[0000, 0]@13..33#1
            IDENT   start_span 42:Root[0000, 0]@34..34#1
        "#]],
    );
}

#[test]
fn test_fn_like_mk_literals() {
    assert_expand(
        "fn_like_mk_literals",
        r#""#,
        expect![[r#"


            LITERAL ByteStr byte_string 1
            LITERAL Char c 1
            LITERAL Str string 1
            LITERAL Str -string 1
            LITERAL CStr cstring 1
            LITERAL Float 3.14f64 1
            LITERAL Float -3.14f64 1
            LITERAL Float 3.14 1
            LITERAL Float -3.14 1
            LITERAL Integer 123i64 1
            LITERAL Integer -123i64 1
            LITERAL Integer 123 1
            LITERAL Integer -123 1
        "#]],
        expect![[r#"


            LITERAL ByteStr byte_string 42:Root[0000, 0]@0..100#1
            LITERAL Char c 42:Root[0000, 0]@0..100#1
            LITERAL Str string 42:Root[0000, 0]@0..100#1
            LITERAL Str -string 42:Root[0000, 0]@0..100#1
            LITERAL CStr cstring 42:Root[0000, 0]@0..100#1
            LITERAL Float 3.14f64 42:Root[0000, 0]@0..100#1
            LITERAL Float -3.14f64 42:Root[0000, 0]@0..100#1
            LITERAL Float 3.14 42:Root[0000, 0]@0..100#1
            LITERAL Float -3.14 42:Root[0000, 0]@0..100#1
            LITERAL Integer 123i64 42:Root[0000, 0]@0..100#1
            LITERAL Integer -123i64 42:Root[0000, 0]@0..100#1
            LITERAL Integer 123 42:Root[0000, 0]@0..100#1
            LITERAL Integer -123 42:Root[0000, 0]@0..100#1
        "#]],
    );
}

#[test]
fn test_fn_like_mk_idents() {
    assert_expand(
        "fn_like_mk_idents",
        r#""#,
        expect![[r#"


            IDENT   standard 1
            IDENT   r#raw 1
        "#]],
        expect![[r#"


            IDENT   standard 42:Root[0000, 0]@0..100#1
            IDENT   r#raw 42:Root[0000, 0]@0..100#1
        "#]],
    );
}

#[test]
fn test_fn_like_macro_clone_literals() {
    assert_expand(
        "fn_like_clone_tokens",
        r###"1u16, 2_u32, -4i64, 3.14f32, "hello bridge", "suffixed"suffix, r##"raw"##, 'a', b'b', c"null""###,
        expect![[r#"
            LITERAL Integer 1u16 1
            PUNCT   , [alone] 1
            LITERAL Integer 2_u32 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Integer 4i64 1
            PUNCT   , [alone] 1
            LITERAL Float 3.14f32 1
            PUNCT   , [alone] 1
            LITERAL Str hello bridge 1
            PUNCT   , [alone] 1
            LITERAL Err(()) "suffixed"suffix 1
            PUNCT   , [alone] 1
            LITERAL StrRaw(2) raw 1
            PUNCT   , [alone] 1
            LITERAL Char a 1
            PUNCT   , [alone] 1
            LITERAL Byte b 1
            PUNCT   , [alone] 1
            LITERAL CStr null 1


            LITERAL Integer 1u16 1
            PUNCT   , [alone] 1
            LITERAL Integer 2_u32 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Integer 4i64 1
            PUNCT   , [alone] 1
            LITERAL Float 3.14f32 1
            PUNCT   , [alone] 1
            LITERAL Str hello bridge 1
            PUNCT   , [alone] 1
            LITERAL Err(()) "suffixed"suffix 1
            PUNCT   , [alone] 1
            LITERAL StrRaw(2) raw 1
            PUNCT   , [alone] 1
            LITERAL Char a 1
            PUNCT   , [alone] 1
            LITERAL Byte b 1
            PUNCT   , [alone] 1
            LITERAL CStr null 1
        "#]],
        expect![[r#"
            LITERAL Integer 1u16 42:Root[0000, 0]@0..4#1
            PUNCT   , [alone] 42:Root[0000, 0]@4..5#1
            LITERAL Integer 2_u32 42:Root[0000, 0]@6..11#1
            PUNCT   , [alone] 42:Root[0000, 0]@11..12#1
            PUNCT   - [alone] 42:Root[0000, 0]@13..14#1
            LITERAL Integer 4i64 42:Root[0000, 0]@14..18#1
            PUNCT   , [alone] 42:Root[0000, 0]@18..19#1
            LITERAL Float 3.14f32 42:Root[0000, 0]@20..27#1
            PUNCT   , [alone] 42:Root[0000, 0]@27..28#1
            LITERAL Str hello bridge 42:Root[0000, 0]@29..43#1
            PUNCT   , [alone] 42:Root[0000, 0]@43..44#1
            LITERAL Err(()) "suffixed"suffix 42:Root[0000, 0]@45..61#1
            PUNCT   , [alone] 42:Root[0000, 0]@61..62#1
            LITERAL StrRaw(2) raw 42:Root[0000, 0]@63..73#1
            PUNCT   , [alone] 42:Root[0000, 0]@73..74#1
            LITERAL Char a 42:Root[0000, 0]@75..78#1
            PUNCT   , [alone] 42:Root[0000, 0]@78..79#1
            LITERAL Byte b 42:Root[0000, 0]@80..84#1
            PUNCT   , [alone] 42:Root[0000, 0]@84..85#1
            LITERAL CStr null 42:Root[0000, 0]@86..93#1


            LITERAL Integer 1u16 42:Root[0000, 0]@0..4#1
            PUNCT   , [alone] 42:Root[0000, 0]@4..5#1
            LITERAL Integer 2_u32 42:Root[0000, 0]@6..11#1
            PUNCT   , [alone] 42:Root[0000, 0]@11..12#1
            PUNCT   - [alone] 42:Root[0000, 0]@13..14#1
            LITERAL Integer 4i64 42:Root[0000, 0]@14..18#1
            PUNCT   , [alone] 42:Root[0000, 0]@18..19#1
            LITERAL Float 3.14f32 42:Root[0000, 0]@20..27#1
            PUNCT   , [alone] 42:Root[0000, 0]@27..28#1
            LITERAL Str hello bridge 42:Root[0000, 0]@29..43#1
            PUNCT   , [alone] 42:Root[0000, 0]@43..44#1
            LITERAL Err(()) "suffixed"suffix 42:Root[0000, 0]@45..61#1
            PUNCT   , [alone] 42:Root[0000, 0]@61..62#1
            LITERAL StrRaw(2) raw 42:Root[0000, 0]@63..73#1
            PUNCT   , [alone] 42:Root[0000, 0]@73..74#1
            LITERAL Char a 42:Root[0000, 0]@75..78#1
            PUNCT   , [alone] 42:Root[0000, 0]@78..79#1
            LITERAL Byte b 42:Root[0000, 0]@80..84#1
            PUNCT   , [alone] 42:Root[0000, 0]@84..85#1
            LITERAL CStr null 42:Root[0000, 0]@86..93#1
        "#]],
    );
}

#[test]
fn test_fn_like_macro_negative_literals() {
    assert_expand(
        "fn_like_clone_tokens",
        r###"-1u16, - 2_u32, -3.14f32, - 2.7"###,
        expect![[r#"
            PUNCT   - [alone] 1
            LITERAL Integer 1u16 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Integer 2_u32 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Float 3.14f32 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Float 2.7 1


            PUNCT   - [alone] 1
            LITERAL Integer 1u16 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Integer 2_u32 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Float 3.14f32 1
            PUNCT   , [alone] 1
            PUNCT   - [alone] 1
            LITERAL Float 2.7 1
        "#]],
        expect![[r#"
            PUNCT   - [alone] 42:Root[0000, 0]@0..1#1
            LITERAL Integer 1u16 42:Root[0000, 0]@1..5#1
            PUNCT   , [alone] 42:Root[0000, 0]@5..6#1
            PUNCT   - [alone] 42:Root[0000, 0]@7..8#1
            LITERAL Integer 2_u32 42:Root[0000, 0]@9..14#1
            PUNCT   , [alone] 42:Root[0000, 0]@14..15#1
            PUNCT   - [alone] 42:Root[0000, 0]@16..17#1
            LITERAL Float 3.14f32 42:Root[0000, 0]@17..24#1
            PUNCT   , [alone] 42:Root[0000, 0]@24..25#1
            PUNCT   - [alone] 42:Root[0000, 0]@26..27#1
            LITERAL Float 2.7 42:Root[0000, 0]@28..31#1


            PUNCT   - [alone] 42:Root[0000, 0]@0..1#1
            LITERAL Integer 1u16 42:Root[0000, 0]@1..5#1
            PUNCT   , [alone] 42:Root[0000, 0]@5..6#1
            PUNCT   - [alone] 42:Root[0000, 0]@7..8#1
            LITERAL Integer 2_u32 42:Root[0000, 0]@9..14#1
            PUNCT   , [alone] 42:Root[0000, 0]@14..15#1
            PUNCT   - [alone] 42:Root[0000, 0]@16..17#1
            LITERAL Float 3.14f32 42:Root[0000, 0]@17..24#1
            PUNCT   , [alone] 42:Root[0000, 0]@24..25#1
            PUNCT   - [alone] 42:Root[0000, 0]@26..27#1
            LITERAL Float 2.7 42:Root[0000, 0]@28..31#1
        "#]],
    );
}

#[test]
fn test_attr_macro() {
    // Corresponds to
    //    #[proc_macro_test::attr_error(some arguments)]
    //    mod m {}
    assert_expand_attr(
        "attr_error",
        r#"mod m {}"#,
        r#"some arguments"#,
        expect![[r#"
            IDENT   mod 1
            IDENT   m 1
            GROUP {} 1 1


            IDENT   some 1
            IDENT   arguments 1


            IDENT   compile_error 1
            PUNCT   ! [joint] 1
            GROUP () 1 1
              LITERAL Str #[attr_error(some arguments)] mod m {} 1
            PUNCT   ; [alone] 1
        "#]],
        expect![[r#"
            IDENT   mod 42:Root[0000, 0]@0..3#1
            IDENT   m 42:Root[0000, 0]@4..5#1
            GROUP {} 42:Root[0000, 0]@6..7#1 42:Root[0000, 0]@7..8#1


            IDENT   some 42:Root[0000, 0]@0..4#1
            IDENT   arguments 42:Root[0000, 0]@5..14#1


            IDENT   compile_error 42:Root[0000, 0]@0..13#1
            PUNCT   ! [joint] 42:Root[0000, 0]@13..14#1
            GROUP () 42:Root[0000, 0]@14..15#1 42:Root[0000, 0]@55..56#1
              LITERAL Str #[attr_error(some arguments)] mod m {} 42:Root[0000, 0]@15..55#1
            PUNCT   ; [alone] 42:Root[0000, 0]@56..57#1
        "#]],
    );
}

#[test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: \"Mismatched token groups\""]
fn test_broken_input_unclosed_delim() {
    assert_expand("fn_like_clone_tokens", r###"{"###, expect![[]], expect![[]]);
}

#[test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: \"Unexpected '}'\""]
fn test_broken_input_unopened_delim() {
    assert_expand("fn_like_clone_tokens", r###"}"###, expect![[]], expect![[]]);
}

#[test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: \"Expected '}'\""]
fn test_broken_input_mismatched_delim() {
    assert_expand("fn_like_clone_tokens", r###"(}"###, expect![[]], expect![[]]);
}

#[test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: \"Invalid identifier: `🪟`\""]
fn test_broken_input_unknowm_token() {
    assert_expand("fn_like_clone_tokens", r###"🪟"###, expect![[]], expect![[]]);
}

/// Tests that we find and classify all proc macros correctly.
#[test]
fn list_test_macros() {
    let res = list().join("\n");

    expect![[r#"
        fn_like_noop [Bang]
        fn_like_panic [Bang]
        fn_like_error [Bang]
        fn_like_clone_tokens [Bang]
        fn_like_mk_literals [Bang]
        fn_like_mk_idents [Bang]
        fn_like_span_join [Bang]
        fn_like_span_ops [Bang]
        fn_like_span_line_column [Bang]
        attr_noop [Attr]
        attr_panic [Attr]
        attr_error [Attr]
        DeriveReemit [CustomDerive]
        DeriveEmpty [CustomDerive]
        DerivePanic [CustomDerive]
        DeriveError [CustomDerive]"#]]
    .assert_eq(&res);
}

#[test]
fn test_fn_like_span_line_column() {
    assert_expand_with_callback(
        "fn_like_span_line_column",
        // Input text with known position: "hello" starts at offset 1 (line 2, column 1 in 1-based)
        "
hello",
        expect![[r#"
            LITERAL Integer 2 42:Root[0000, 0]@0..100#1
            LITERAL Integer 1 42:Root[0000, 0]@0..100#1
        "#]],
    );
}
