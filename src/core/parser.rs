use std::str;
use std::str::FromStr;

extern crate nom;
use self::nom::{alphanumeric, digit, space, IResult};

extern crate regex;

use core::pbrt::Float;
use core::paramset::{ParamList, ParamSet, ParamSetItem, Point2f, Point3f, Value};

#[derive(PartialEq, Debug)]
pub enum Error {
    NomError(nom::Err),
    NomIncomplete(nom::Needed),
}

#[derive(Debug, Clone, PartialEq)]
enum OptionsBlock {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    // TODO(wathiede): convert to 3 x Vector3f?
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

#[derive(Debug, Clone, PartialEq)]
enum WorldBlock {
    Attribute(Vec<WorldBlock>), // Used for holding world block objects between AttributeBegin/End blocks.
    LightSource(String, ParamSet),
    Material(String, ParamSet),
    Shape(String, ParamSet),
    // TODO(wathiede): convert to Vector3f?
    Translate(Float, Float, Float),
    Texture(
        String, // name
        String, // type
        String, // class
        ParamSet,
    ),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    options: Vec<OptionsBlock>,
    world_objects: Vec<WorldBlock>,
}

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

named!(
    parse_point3f<Point3f>,
    ws!(do_parse!(
        x: number >> y: number >> z: number >> (Point3f { x, y, z })
    ))
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_point<Value>,
    ws!(
        do_parse!(
                tag!("[") >>
                points: many1!(parse_point3f) >>
                tag!("]") >>
            (Value::Point3f(points.into()))
        )
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
named!(param_set_item_values_rgb<Value>,
    do_parse!(
        values: alt!(
            ws!(delimited!(tag!("["), many1!(number), tag!("]")))
            | ws!(many1!(number))
        ) >>
        (Value::RGB(ParamList(values)))
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_blackbody<Value>,
    do_parse!(
        values: alt!(
            ws!(delimited!(tag!("["), many1!(number), tag!("]")))
            | ws!(many1!(number))
        ) >>
        (Value::Blackbody(ParamList(values)))
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

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set_item_values_texture<Value>,
    alt!(
        do_parse!(
            values: ws!(delimited!(tag!("["), many1!(ascii), tag!("]"))) >>
            (Value::Texture(ParamList(values)))
        ) |
        do_parse!(
            value: ws!(ascii) >>
            (Value::Texture(ParamList(vec![value])))
        )
    )
);

fn param_set_item_values<'a, 'b>(input: &'a [u8], psi_type: &'b [u8]) -> IResult<&'a [u8], Value> {
    match psi_type {
        b"bool" => param_set_item_values_bool(input),
        b"float" => param_set_item_values_float(input),
        b"integer" => param_set_item_values_integer(input),
        b"string" => param_set_item_values_string(input),
        b"point" => param_set_item_values_point(input),
        b"rgb" => param_set_item_values_rgb(input),
        b"texture" => param_set_item_values_texture(input),
        b"blackbody" => param_set_item_values_blackbody(input),
        _ => panic!(format!(
            "unhandled param_set_item {:?}",
            str::from_utf8(psi_type).unwrap()
        )),
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    param_set_item<ParamSetItem>,
    do_parse!(
        tag!("\"") >>
        typ: alphanumeric >>
        space >>
        name: map_res!(alphanumeric, str::from_utf8) >>
        tag!("\"") >>
        values: call!(param_set_item_values, typ) >>
        (ParamSetItem::new(name, values))
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(param_set<Vec<ParamSetItem>>,
    many0!(param_set_item)
);

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

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    camera<OptionsBlock>,
    ws!(
        do_parse!(
            tag!("Camera") >>
            name: quoted_name >>
            ps: param_set >>
            (OptionsBlock::Camera(name.into(), ps.into()))
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
            (OptionsBlock::Sampler(name.into(), ps.into()))
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
            (OptionsBlock::Integrator(name.into(), ps.into()))
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
            (OptionsBlock::Film(name.into(), ps.into()))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    option_block<Vec<OptionsBlock>>,
    dbg_dmp!(many1!(
        alt!(
            film
            | look_at
            | camera
            | sampler
            | integrator
        )
    )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    attribute<WorldBlock>,
    ws!(
        do_parse!(
            tag!("AttributeBegin") >>
            world_objects: world_objects >>
            tag!("AttributeEnd") >>
            (WorldBlock::Attribute(world_objects))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    light_source<WorldBlock>,
    ws!(
        do_parse!(
            tag!("LightSource") >>
            name: quoted_name >>
            ps: param_set >>
            (WorldBlock::LightSource(name.into(), ps.into()))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    material<WorldBlock>,
    ws!(
        do_parse!(
            tag!("Material") >>
            name: quoted_name >>
            ps: param_set >>
            (WorldBlock::Material(name.into(), ps.into()))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    shape<WorldBlock>,
    ws!(
        do_parse!(
            tag!("Shape") >>
            name: quoted_name >>
            ps: param_set >>
            (WorldBlock::Shape(name.into(), ps.into()))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    translate<WorldBlock>,
    ws!(
        do_parse!(
            tag!("Translate") >>
            x: number >>
            y: number >>
            z: number >>
            (WorldBlock::Translate(x, y, z))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    texture<WorldBlock>,
    ws!(
        do_parse!(
            tag!("Texture") >>
            name: quoted_name >>
            typ: quoted_name >>
            class: quoted_name >>
            ps: param_set >>
            (WorldBlock::Texture(name.into(), typ.into(), class.into(), ps.into()))
        )
    )
);

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    world_objects<Vec<WorldBlock>>,
    many1!(
        alt!(
            shape
            | texture
            | material
            | attribute
            | translate
            | light_source
        )
    )
);

pub fn parse_scene(input: &[u8]) -> Result<Scene, Error> {
    match strip_comment(input) {
        // TODO(wathiede): Implement From for IResult and use ? operator here.
        IResult::Done(_, i) => match parse_scene_macro(&i) {
            IResult::Done(_, scene) => Ok(scene),
            IResult::Error(e) => Err(Error::NomError(e)),
            IResult::Incomplete(n) => Err(Error::NomIncomplete(n)),
        },
        IResult::Error(e) => Err(Error::NomError(e)),
        IResult::Incomplete(n) => Err(Error::NomIncomplete(n)),
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
named!(
    parse_scene_macro<Scene>,
    ws!(
        do_parse!(
            options: option_block >>
            tag!("WorldBegin") >>
            world_objects: world_objects >>
            tag!("WorldEnd") >>
            (Scene{options, world_objects})
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
    fn test_param_set_item_values_point() {
        let input = &b"[.1 .2 .3  .4 .5 .6]\n"[..];
        let ref res = param_set_item_values_point(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                Value::Point3f(
                    vec![
                        Point3f {
                            x: 0.1,
                            y: 0.2,
                            z: 0.3,
                        },
                        Point3f {
                            x: 0.4,
                            y: 0.5,
                            z: 0.6,
                        },
                    ].into()
                )
            )
        );
    }

    #[test]
    fn test_param_set_item_values_float() {
        let input = &b"[.4 .45 .5]\n"[..];
        let ref res = param_set_item_values_float(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Float(ParamList(vec![0.4, 0.45, 0.5])))
        );

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
    fn test_param_set_item_values_texture() {
        let input = &b"[\"foo\"]\n"[..];
        let ref res = param_set_item_values_texture(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Texture(ParamList(vec!["foo".to_owned()])))
        );

        let input = &b"\"foo\"\n"[..];
        let ref res = param_set_item_values_texture(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], Value::Texture(ParamList(vec!["foo".to_owned()])))
        );
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
            assert_eq!(
                res,
                &IResult::Done(
                    &b""[..],
                    OptionsBlock::Camera(
                        "perspective".into(),
                        vec![ParamSetItem::new("fov", Value::Float(vec![45.].into()))].into()
                    ),
                )
            );
        };
    }

    #[test]
    fn test_sampler() {
        let input = &b"Sampler \"halton\" \"integer pixelsamples\" 128"[..];
        if let IResult::Done(_, input) = strip_comment(input) {
            let ref mut res = sampler(&input);
            assert_eq!(
                res,
                &IResult::Done(
                    &b""[..],
                    OptionsBlock::Sampler(
                        "halton".into(),
                        vec![
                            ParamSetItem::new("pixelsamples", Value::Int(vec![128].into())),
                        ].into()
                    )
                )
            );
        };
    }

    #[test]
    fn test_integrator() {
        let input = &b"Integrator \"path\""[..];
        if let IResult::Done(_, input) = strip_comment(input) {
            let ref mut res = integrator(&input);
            assert_eq!(
                res,
                &IResult::Done(
                    &b""[..],
                    OptionsBlock::Integrator("path".into(), vec![].into())
                )
            );
        };
    }

    #[test]
    fn test_film() {
        let input = &b"Film \"image\" \"string filename\" \"simple.png\"
\"integer xresolution\" [400] \"integer yresolution\" [200]"[..];
        let ref mut res = film(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                OptionsBlock::Film(
                    "image".into(),
                    vec![
                        ParamSetItem::new(
                            "filename",
                            Value::String(vec!["simple.png".to_owned()].into()),
                        ),
                        ParamSetItem::new("xresolution", Value::Int(vec![400].into())),
                        ParamSetItem::new("yresolution", Value::Int(vec![200].into())),
                    ].into()
                )
            )
        );
    }

    #[test]
    fn test_attribute() {
        let input =
            &b"AttributeBegin\n  LightSource \"infinite\" \"rgb L\" [.4 .45 .5]\nAttributeEnd"[..];
        let ref mut res = attribute(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::Attribute(vec![
                    WorldBlock::LightSource(
                        "infinite".into(),
                        vec![
                            ParamSetItem::new("L", Value::RGB(ParamList(vec![0.4, 0.45, 0.5]))),
                        ].into(),
                    ),
                ])
            )
        );

        let input = &b"AttributeBegin\n  Material \"mirror\"\nAttributeEnd"[..];
        let ref mut res = attribute(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::Attribute(vec![WorldBlock::Material("mirror".into(), vec![].into())])
            )
        );

        let input = &b"AttributeBegin\n  Shape \"sphere\" \"float radius\" 1\nAttributeEnd"[..];
        let ref mut res = attribute(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::Attribute(
                    vec![
                        WorldBlock::Shape(
                            "sphere".into(),
                            vec![ParamSetItem::new("radius", Value::Float(vec![1.].into()))].into(),
                        ),
                    ].into(),
                ),
            )
        );

        let input = &b"AttributeBegin\n  Material \"mirror\"\n  Shape \"sphere\" \"float radius\" 1\nAttributeEnd"[..];
        let ref mut res = attribute(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::Attribute(
                    vec![
                        WorldBlock::Material("mirror".into(), vec![].into()),
                        WorldBlock::Shape(
                            "sphere".into(),
                            vec![ParamSetItem::new("radius", Value::Float(vec![1.].into()))].into(),
                        ),
                    ].into(),
                ),
            )
        );
    }

    #[test]
    fn test_light_source() {
        let input = &b"LightSource \"infinite\" \"rgb L\" [.4 .45 .5]"[..];
        let ref mut res = light_source(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::LightSource(
                    "infinite".into(),
                    vec![
                        ParamSetItem::new("L", Value::RGB(ParamList(vec![0.4, 0.45, 0.5]))),
                    ].into()
                )
            )
        );
    }

    #[test]
    fn test_material() {
        let input = &b"Material \"mirror\""[..];
        let ref mut res = material(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::Material("mirror".into(), vec![].into()),
            )
        );
    }

    #[test]
    fn test_shape() {
        let input = &b"Shape \"sphere\" \"float radius\" 1"[..];
        let ref mut res = shape(&input);
        assert_eq!(
            res,
            &IResult::Done(
                &b""[..],
                WorldBlock::Shape(
                    "sphere".into(),
                    vec![ParamSetItem::new("radius", Value::Float(vec![1.].into()))].into(),
                ),
            )
        );
    }

    #[test]
    fn test_translate() {
        let input = &b"Translate 0 0 -1"[..];
        let ref mut res = translate(&input);
        assert_eq!(
            res,
            &IResult::Done(&b""[..], WorldBlock::Translate(0., 0., -1.))
        );
    }

    #[test]
    fn test_parse_scene() {
        let input = include_bytes!("testdata/scene1.pbrt");
        let res = parse_scene(&input[..]);
        let want = Scene {
            options: vec![
                OptionsBlock::LookAt(3., 4., 1.5, 0.5, 0.5, 0., 0., 0., 1.),
                OptionsBlock::Camera(
                    "perspective".into(),
                    vec![ParamSetItem::new("fov", Value::Float(vec![45.].into()))].into(),
                ),
                OptionsBlock::Sampler(
                    "halton".into(),
                    vec![
                        ParamSetItem::new("pixelsamples", Value::Int(vec![128].into())),
                    ].into(),
                ),
                OptionsBlock::Integrator("path".into(), vec![].into()),
                OptionsBlock::Film(
                    "image".into(),
                    vec![
                        ParamSetItem::new(
                            "filename",
                            Value::String(vec!["simple.png".to_owned()].into()),
                        ),
                        ParamSetItem::new("xresolution", Value::Int(vec![400].into())),
                        ParamSetItem::new("yresolution", Value::Int(vec![300].into())),
                    ].into(),
                ),
            ],
            world_objects: vec![
                WorldBlock::LightSource(
                    "infinite".into(),
                    vec![
                        ParamSetItem::new("L", Value::RGB(vec![0.4, 0.45, 0.5].into())),
                    ].into(),
                ),
                WorldBlock::LightSource(
                    "distant".into(),
                    vec![
                        ParamSetItem::new(
                            "from",
                            Value::Point3f(
                                vec![
                                    Point3f {
                                        x: -30.,
                                        y: 40.,
                                        z: 100.,
                                    },
                                ].into(),
                            ),
                        ),
                        ParamSetItem::new("L", Value::Blackbody(vec![3000., 1.5].into())),
                    ].into(),
                ),
                WorldBlock::Attribute(vec![
                    WorldBlock::Material("mirror".into(), vec![].into()),
                    WorldBlock::Shape(
                        "sphere".into(),
                        vec![ParamSetItem::new("radius", Value::Float(vec![1.].into()))].into(),
                    ),
                ]),
                WorldBlock::Attribute(vec![
                    WorldBlock::Texture(
                        "checks".into(),
                        "spectrum".into(),
                        "checkerboard".into(),
                        vec![
                            ParamSetItem::new("uscale", Value::Float(vec![8.].into())),
                            ParamSetItem::new("vscale", Value::Float(vec![8.].into())),
                            ParamSetItem::new("tex1", Value::RGB(ParamList(vec![0.1, 0.1, 0.1]))),
                            ParamSetItem::new("tex2", Value::RGB(ParamList(vec![0.8, 0.8, 0.8]))),
                        ].into(),
                    ),
                    WorldBlock::Material(
                        "matte".into(),
                        vec![
                            ParamSetItem::new("Kd", Value::Texture(vec!["checks".into()].into())),
                        ].into(),
                    ),
                    WorldBlock::Translate(0., 0., -1.),
                    WorldBlock::Shape(
                        "trianglemesh".into(),
                        vec![
                            ParamSetItem::new("indices", Value::Int(vec![0, 1, 2, 0, 2, 3].into())),
                            ParamSetItem::new(
                                "P",
                                Value::Point3f(
                                    vec![
                                        Point3f {
                                            x: -20.,
                                            y: -20.,
                                            z: 0.,
                                        },
                                        Point3f {
                                            x: 20.,
                                            y: -20.,
                                            z: 0.,
                                        },
                                        Point3f {
                                            x: 20.,
                                            y: 20.,
                                            z: 0.,
                                        },
                                        Point3f {
                                            x: -20.,
                                            y: 20.,
                                            z: 0.,
                                        },
                                    ].into(),
                                ),
                            ),
                            ParamSetItem::new(
                                "st",
                                Value::Float(vec![0., 0., 1., 0., 1., 1., 0., 1.].into()),
                            ),
                        ].into(),
                    ),
                ]),
            ],
        };
        assert_eq!(res, Ok(want));
    }
}
/*
Shape "trianglemesh"
    "integer indices" [0 1 2 0 2 3]
    "point P" [ -20 -20 0   20 -20 0   20 20 0   -20 20 0 ]
    "float st" [ 0 0   1 0    1 1   0 1 ]

 */
