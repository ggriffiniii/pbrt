use std::convert;
use std::str;
use std::str::FromStr;

extern crate nom;
use self::nom::{alphanumeric, digit, double, double_s, is_digit, rest, space, ErrorKind, IResult,
                InputLength, Slice};

extern crate regex;

use core::api::Pbrt;
use core::pbrt::Float;
use core::paramset::{ParamList, ParamSet, ParamSetItem, Value};

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(quoted_name<&str>,
   map_res!(
       delimited!(tag!("\""), alphanumeric, tag!("\"")),
       str::from_utf8
   )
);

fn bool(input: &[u8]) -> IResult<&[u8], bool> {
    flat_map!(
        input,
        recognize!(alt!(tag!("true") | tag!("false"))),
        parse_to!(bool)
    )
}

// number is a superset of nom::double! that includes '1' and '1.'
fn number(input: &[u8]) -> IResult<&[u8], Float> {
    flat_map!(
        input,
        recognize!(tuple!(
            opt!(alt!(tag!("+") | tag!("-"))),
            alt!(
                complete!(delimited!(opt!(digit), tag!("."), digit))
                    | complete!(preceded!(digit, tag!("."))) | digit
            ),
            opt!(complete!(tuple!(
                alt!(tag!("e") | tag!("E")),
                opt!(alt!(tag!("+") | tag!("-"))),
                digit
            )))
        )),
        parse_to!(Float)
    )
}

fn integer(input: &[u8]) -> IResult<&[u8], i64> {
    flat_map!(
        input,
        recognize!(tuple!(opt!(alt!(tag!("+") | tag!("-"))), complete!(digit))),
        parse_to!(i64)
    )
}

fn strip_comment(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    // TODO(wathiede): handle "#" if necessary.
    let re = regex::bytes::Regex::new(r"#.*\n").unwrap();
    let res = re.replace_all(input, &b"\n"[..]);

    IResult::Done(&b""[..], Vec::from(res))
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_bool<Value>,
    do_parse!(
        values: alt!(
            ws!(delimited!(tag!("["), many1!(bool), tag!("]")))
            | ws!(many1!(bool))
        ) >>
        (Value::Bool(ParamList(values)))
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_float<Value>,
    do_parse!(
        values: alt!(
            ws!(delimited!(tag!("["), many1!(number), tag!("]")))
            | ws!(many1!(number))
        ) >>
        (Value::Float(ParamList(values)))
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_integer<Value>,
    do_parse!(
        values: alt!(
            ws!(delimited!(tag!("["), many1!(integer), tag!("]")))
            | ws!(many1!(integer))
        ) >>
        (Value::Int(ParamList(values)))
    )
);

named!(
    ascii<String>,
    map_res!(
        map_res!(
            delimited!(tag!("\""), take_until!("\""), tag!("\"")),
            str::from_utf8
        ),
        String::from_str
    )
);

// TODO(wathiede): should we applu this pattern to everything, only perform many1! when brackets
// are present?
#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_string<Value>,
    alt!(
        do_parse!(
            values: ws!(delimited!(tag!("["), many1!(ascii), tag!("]"))) >>
            (Value::String(ParamList(values)))
        ) |
        do_parse!(
            value: ws!(ascii) >>
            (Value::String(ParamList(vec![value])))
        )
    )
);

fn param_set_item_values<'a, 'b>(input: &'a [u8], psi_type: &'b [u8]) -> IResult<&'a [u8], Value> {
    match psi_type {
        b"bool" => param_set_item_values_bool(input),
        b"float" => param_set_item_values_float(input),
        b"integer" => param_set_item_values_integer(input),
        b"string" => param_set_item_values_string(input),
        _ => panic!(format!(
            "unhandled param_set_item {:?}",
            str::from_utf8(psi_type)
        )),
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    param_set_item<ParamSetItem>,
    dbg_dmp!(do_parse!(
        tag!("\"") >>
        typ: alphanumeric >>
        space >>
        name: map_res!(alphanumeric, str::from_utf8) >>
        tag!("\"") >>
        values: call!(param_set_item_values, typ) >>
        (ParamSetItem::new(name, values))
    ))
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set<Vec<ParamSetItem>>,
    many0!(param_set_item)
);

#[derive(Debug, Clone, PartialEq)]
enum OptionsBlock {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    LookAt(
        Float, Float, Float, // eye xyz
        Float, Float, Float, // look xyz
        Float, Float, Float, // up xyz
    ),
    Camera(String, ParamSet),
    Sampler(String, ParamSet),
    Integrator(String, ParamSet),
    Film(String, ParamSet),
}

enum WorldBlock {
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    look_at<OptionsBlock>,
    ws!(
        do_parse!(
            tag!("LookAt") >>
            ex: number >>
            ey: number >>
            ez: number >>
            lx: number >>
            ly: number >>
            lz: number >>
            ux: number >>
            uy: number >>
            uz: number >>
            (OptionsBlock::LookAt(ex, ey, ez, lx, ly, lz, ux, uy, uz))
        )
    )
);

fn param_set_from_vec(psis: Vec<ParamSetItem>) -> ParamSet {
    let mut ps = ParamSet::new();
    for ref psi in psis.iter() {
        ps.add(&psi.name, psi.values.clone())
    }
    ps
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    camera<OptionsBlock>,
    ws!(
        do_parse!(
            tag!("Camera") >>
            name: quoted_name >>
            ps: param_set >>
            (OptionsBlock::Camera(String::from(name), param_set_from_vec(ps)))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    sampler<OptionsBlock>,
    ws!(
        do_parse!(
            tag!("Sampler") >>
            name: quoted_name >>
            ps: param_set >>
            (OptionsBlock::Sampler(String::from(name), param_set_from_vec(ps)))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    integrator<OptionsBlock>,
    ws!(
        do_parse!(
            tag!("Integrator") >>
            name: quoted_name >>
            ps: param_set >>
            (OptionsBlock::Integrator(String::from(name), param_set_from_vec(ps)))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    film<OptionsBlock>,
    ws!(
        do_parse!(
            tag!("Film") >>
            name: quoted_name >>
            ps: param_set >>
            (OptionsBlock::Film(String::from(name), param_set_from_vec(ps)))
        )
    )
);

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use nom::IResult;

    use super::*;

    fn dump<O>(res: &IResult<&[u8], O>)
    where
        O: Debug,
    {
        match res {
            &IResult::Done(ref i, ref o) => println!("i: {:?} | o: {:?}", str::from_utf8(i), o),
            &IResult::Error(ref e) => panic!("e: {:#?}", e),
            &IResult::Incomplete(n) => panic!("need: {:?}", n),
        }
    }

    #[test]
    fn test_strip_comment() {
        assert_eq!(
            strip_comment(&b"a # comment\nb # comment\n"[..]),
            IResult::Done(&b""[..], Vec::from(&b"a \nb \n"[..]))
        );
        assert_eq!(
            strip_comment(&b"a # comment\nb\n"[..]),
            IResult::Done(&b""[..], Vec::from(&b"a \nb\n"[..]))
        );
        assert_eq!(
            strip_comment(&b"a\nb\n"[..]),
            IResult::Done(&b""[..], Vec::from(&b"a\nb\n"[..]))
        );
        assert_eq!(
            strip_comment(&b"a\nb # comment\n"[..]),
            IResult::Done(&b""[..], Vec::from(&b"a\nb \n"[..]))
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(number(&b"3.14"[..]), IResult::Done(&b""[..], 3.14));
        assert_eq!(number(&b".1"[..]), IResult::Done(&b""[..], 0.1));
        assert_eq!(number(&b"0.2"[..]), IResult::Done(&b""[..], 0.2));
        assert_eq!(number(&b"3."[..]), IResult::Done(&b""[..], 3.));
        assert_eq!(number(&b"4"[..]), IResult::Done(&b""[..], 4.));
    }

    #[test]
    fn test_number_comment_number() {
        if let IResult::Done(_, input) = strip_comment(&b"[ 1 # comment\n2 3]\n"[..]) {
            let ref res = param_set_item_values_float(&input);
            assert_eq!(
                res,
                &IResult::Done(&b""[..], Value::Float(ParamList(vec![1., 2., 3.])))
            );
        };
    }

    #[test]
    fn test_param_set_item_values_integer() {
        let input = &b"[-1 2 3]\n"[..];
        let ref res = param_set_item_values_integer(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Int(ParamList(vec![-1, 2, 3])))
        );

        let input = &b"[400]\n"[..];
        let ref res = param_set_item_values_integer(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Int(ParamList(vec![400])))
        );
    }

    #[test]
    fn test_param_set_item_values_string() {
        let input = &b"[\"foo\"]\n"[..];
        let ref res = param_set_item_values_string(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::String(ParamList(vec!["foo".to_owned()])))
        );

        let input = &b"\"foo\"\n"[..];
        let ref res = param_set_item_values_string(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::String(ParamList(vec!["foo".to_owned()])))
        );
    }

    #[test]
    fn test_param_set_item_values() {
        let input = &b"1 2. -3.0"[..];
        let ref res = param_set_item_values_float(input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Float(ParamList(vec![1., 2., -3.])))
        );

        let input = &b"[  1 2 3]"[..];
        let ref res = param_set_item_values_float(input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Float(ParamList(vec![1., 2., 3.])))
        );

        // TODO(wathiede): support non-float types:
        // "string filename" "foo.exr"
        // "point origin" [ 0 1 2 ]
        // "normal N" [ 0 1 0  0 0 1 ] # array of 2 normal values
        // "bool renderquickly" "true"
    }

    #[test]
    fn test_param_set_item_float() {
        let input = &b"\"float foo\" [ 0 1 2 ]"[..];
        let ref res = param_set_item(input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                ParamSetItem::new("foo", Value::Float(ParamList(vec![0., 1., 2.])))
            )
        );
    }

    #[test]
    fn test_param_set_item_integer() {
        let input = &b"\"integer foo\" [400]"[..];
        let ref res = param_set_item(input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                ParamSetItem::new("foo", Value::Int(ParamList(vec![400])))
            )
        );
    }

    #[test]
    fn test_param_set_item_bool() {
        let input = &b"\"bool foo\" [true false true false]"[..];
        let ref res = param_set_item(input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                ParamSetItem::new(
                    "foo",
                    Value::Bool(ParamList(vec![true, false, true, false]))
                )
            )
        );
    }

    #[test]
    fn test_param_set_item_multi() {
        let input = &b"\"bool foo\" [true false true false]
\"integer bar\" [ 0 1 2 ]
"[..];
        let ref res = param_set(input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                vec![
                    ParamSetItem::new(
                        "foo",
                        Value::Bool(ParamList(vec![true, false, true, false])),
                    ),
                    ParamSetItem::new("bar", Value::Int(ParamList(vec![0, 1, 2]))),
                ]
            )
        );

        let input = &b"\"integer xresolution\" [400] \"integer yresolution\" [200]
\"string filename\" \"simple.png\"
"[..];
        let ref res = param_set(input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                vec![
                    ParamSetItem::new("xresolution", Value::Int(ParamList(vec![400]))),
                    ParamSetItem::new("yresolution", Value::Int(ParamList(vec![200]))),
                    ParamSetItem::new(
                        "filename",
                        Value::String(ParamList(vec!["simple.png".to_owned()])),
                    ),
                ]
            )
        );
        let input = &b"\"string filename\" \"simple.png\"
\"integer xresolution\" [400] \"integer yresolution\" [200]"[..];
        let ref res = param_set(input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                vec![
                    ParamSetItem::new(
                        "filename",
                        Value::String(ParamList(vec!["simple.png".to_owned()])),
                    ),
                    ParamSetItem::new("xresolution", Value::Int(ParamList(vec![400]))),
                    ParamSetItem::new("yresolution", Value::Int(ParamList(vec![200]))),
                ]
            )
        );
    }

    #[test]
    fn test_look_at() {
        let input = &b"LookAt 3 4 1.5  # eye
.5 .5 0  # look at point
0 0 1    # up vector
"[..];
        if let IResult::Done(_, input) = strip_comment(input) {
            let ref mut res = look_at(&input);
            assert_eq!(
                res,
                &IResult::Done(
                    &b""[..],
                    OptionsBlock::LookAt(3., 4., 1.5, 0.5, 0.5, 0., 0., 0., 1.)
                )
            );
        };
    }

    #[test]
    fn test_camera() {
        let input = &b"Camera \"perspective\" \"float fov\" 45"[..];
        if let IResult::Done(_, input) = strip_comment(input) {
            let ref mut res = camera(&input);
            let mut ps = ParamSet::new();
            ps.add("fov", Value::Float(ParamList(vec![45.])));
            assert_eq!(
                res,
                &IResult::Done(
                    &b""[..],
                    OptionsBlock::Camera(String::from("perspective"), ps)
                )
            );
        };
    }

    #[test]
    fn test_sampler() {
        let input = &b"Sampler \"halton\" \"integer pixelsamples\" 128"[..];
        if let IResult::Done(_, input) = strip_comment(input) {
            let ref mut res = sampler(&input);
            let mut ps = ParamSet::new();
            ps.add("pixelsamples", Value::Int(ParamList(vec![128])));
            assert_eq!(
                res,
                &IResult::Done(&b""[..], OptionsBlock::Sampler(String::from("halton"), ps))
            );
        };
    }

    #[test]
    fn test_integrator() {
        let input = &b"Integrator \"path\""[..];
        if let IResult::Done(_, input) = strip_comment(input) {
            let ref mut res = integrator(&input);
            let mut ps = ParamSet::new();
            assert_eq!(
                res,
                &IResult::Done(&b""[..], OptionsBlock::Integrator(String::from("path"), ps))
            );
        };
    }

    #[test]
    fn test_film() {
        let input = &b"Film \"image\" \"string filename\" \"simple.png\"
\"integer xresolution\" [400] \"integer yresolution\" [200]"[..];
        let ref mut res = film(&input);
        let mut ps = ParamSet::new();
        ps.add(
            "filename",
            Value::String(ParamList(vec!["simple.png".to_owned()])),
        );
        ps.add("xresolution", Value::Int(ParamList(vec![400])));
        ps.add("yresolution", Value::Int(ParamList(vec![200])));
        dump(res);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], OptionsBlock::Film(String::from("image"), ps))
        );
    }
}
