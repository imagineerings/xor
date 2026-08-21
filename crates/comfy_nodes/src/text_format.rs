use std::collections::BTreeMap;

use comfy_types::CancellationToken;
use serde_json::Value;
use thiserror::Error;

use crate::{NativePrimitive, NativeValue};

pub const NATIVE_TEXT_FORMAT_MAX_TEMPLATE_BYTES: usize = 1024 * 1024;
pub const NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES: usize = 16 * 1024 * 1024;
const NATIVE_TEXT_FORMAT_MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum NativeTextFormatError {
    #[error("text format template is {actual_bytes} bytes, above the {maximum_bytes}-byte limit")]
    TemplateTooLarge {
        actual_bytes: usize,
        maximum_bytes: usize,
    },
    #[error("text format result exceeded the {maximum_bytes}-byte limit")]
    ResultTooLarge { maximum_bytes: usize },
    #[error("invalid text format template: {0}")]
    InvalidTemplate(String),
    #[error("text format field `{0}` is missing")]
    MissingField(String),
    #[error("text format field access is invalid: {0}")]
    InvalidFieldAccess(String),
    #[error("text format value is not safely representable: {0}")]
    UnsupportedValue(String),
    #[error("text formatting was cancelled")]
    Cancelled,
}

pub struct NativeTextFormatter;

impl NativeTextFormatter {
    pub fn format(
        template: &str,
        values: &BTreeMap<String, NativeValue>,
        cancellation: &CancellationToken,
    ) -> Result<String, NativeTextFormatError> {
        if template.len() > NATIVE_TEXT_FORMAT_MAX_TEMPLATE_BYTES {
            return Err(NativeTextFormatError::TemplateTooLarge {
                actual_bytes: template.len(),
                maximum_bytes: NATIVE_TEXT_FORMAT_MAX_TEMPLATE_BYTES,
            });
        }
        render_template(template, values, cancellation, 0)
    }
}

fn render_template(
    template: &str,
    values: &BTreeMap<String, NativeValue>,
    cancellation: &CancellationToken,
    depth: usize,
) -> Result<String, NativeTextFormatError> {
    check_cancellation(cancellation)?;
    if depth > NATIVE_TEXT_FORMAT_MAX_DEPTH {
        return Err(NativeTextFormatError::InvalidTemplate(
            "nested replacement fields are too deep".to_owned(),
        ));
    }
    let mut output = String::new();
    let mut position = 0usize;
    while position < template.len() {
        check_cancellation(cancellation)?;
        let remainder = template.get(position..).ok_or_else(invalid_utf8_boundary)?;
        let character = remainder.chars().next().ok_or_else(invalid_utf8_boundary)?;
        match character {
            '{' if remainder.starts_with("{{") => {
                append_bounded(&mut output, "{")?;
                position += 2;
            }
            '}' if remainder.starts_with("}}") => {
                append_bounded(&mut output, "}")?;
                position += 2;
            }
            '{' => {
                let (field, next_position) = replacement_field(template, position + 1)?;
                let parsed = ParsedField::checked(field)?;
                let resolved = resolve_field(parsed.field_name, values)?;
                let spec = if parsed.format_spec.is_empty() {
                    String::new()
                } else {
                    render_template(parsed.format_spec, values, cancellation, depth + 1)?
                };
                append_bounded(
                    &mut output,
                    &format_resolved(resolved, parsed.conversion, &spec)?,
                )?;
                position = next_position;
            }
            '}' => {
                return Err(NativeTextFormatError::InvalidTemplate(
                    "single closing brace".to_owned(),
                ));
            }
            _ => {
                let mut encoded = [0u8; 4];
                append_bounded(&mut output, character.encode_utf8(&mut encoded))?;
                position += character.len_utf8();
            }
        }
    }
    check_cancellation(cancellation)?;
    Ok(output)
}

fn invalid_utf8_boundary() -> NativeTextFormatError {
    NativeTextFormatError::InvalidTemplate("template boundary was not valid UTF-8".to_owned())
}

fn replacement_field(template: &str, start: usize) -> Result<(&str, usize), NativeTextFormatError> {
    let mut nested = 0usize;
    let mut position = start;
    while position < template.len() {
        let remainder = template.get(position..).ok_or_else(invalid_utf8_boundary)?;
        let character = remainder.chars().next().ok_or_else(invalid_utf8_boundary)?;
        match character {
            '{' => nested = nested.saturating_add(1),
            '}' if nested == 0 => {
                return Ok((
                    template
                        .get(start..position)
                        .ok_or_else(invalid_utf8_boundary)?,
                    position + 1,
                ));
            }
            '}' => nested -= 1,
            _ => {}
        }
        position += character.len_utf8();
    }
    Err(NativeTextFormatError::InvalidTemplate(
        "replacement field is unterminated".to_owned(),
    ))
}

struct ParsedField<'a> {
    field_name: &'a str,
    conversion: Option<char>,
    format_spec: &'a str,
}

impl<'a> ParsedField<'a> {
    fn checked(field: &'a str) -> Result<Self, NativeTextFormatError> {
        let mut bracket_depth = 0usize;
        let mut conversion = None;
        let mut format = None;
        for (position, character) in field.char_indices() {
            match character {
                '[' => bracket_depth = bracket_depth.saturating_add(1),
                ']' if bracket_depth > 0 => bracket_depth -= 1,
                '!' if bracket_depth == 0 && conversion.is_none() => conversion = Some(position),
                ':' if bracket_depth == 0 => {
                    format = Some(position);
                    break;
                }
                _ => {}
            }
        }
        let field_end = conversion.or(format).unwrap_or(field.len());
        let field_name = field.get(..field_end).unwrap_or_default();
        if field_name.is_empty() {
            return Err(NativeTextFormatError::InvalidTemplate(
                "keyword-only formatting requires a field name".to_owned(),
            ));
        }
        let conversion = if let Some(position) = conversion {
            let end = format.unwrap_or(field.len());
            match field.get(position + 1..end).unwrap_or_default() {
                "s" => Some('s'),
                "r" => Some('r'),
                "a" => Some('a'),
                _ => {
                    return Err(NativeTextFormatError::InvalidTemplate(
                        "conversion must be exactly !s, !r, or !a".to_owned(),
                    ));
                }
            }
        } else {
            None
        };
        let format_spec = format
            .and_then(|position| field.get(position + 1..))
            .unwrap_or_default();
        Ok(Self {
            field_name,
            conversion,
            format_spec,
        })
    }
}

enum ValueRef<'a> {
    Native(&'a NativeValue),
    Json(&'a Value),
    Character(char),
}

fn resolve_field<'a>(
    field: &str,
    values: &'a BTreeMap<String, NativeValue>,
) -> Result<ValueRef<'a>, NativeTextFormatError> {
    let root_end = field.find(['.', '[']).unwrap_or(field.len());
    let root = field.get(..root_end).unwrap_or_default();
    if !valid_identifier(root) {
        return Err(NativeTextFormatError::InvalidFieldAccess(field.to_owned()));
    }
    let mut value = ValueRef::Native(
        values
            .get(root)
            .ok_or_else(|| NativeTextFormatError::MissingField(root.to_owned()))?,
    );
    let mut remainder = field.get(root_end..).unwrap_or_default();
    while !remainder.is_empty() {
        if let Some(attribute) = remainder.strip_prefix('.') {
            let end = attribute.find(['.', '[']).unwrap_or(attribute.len());
            let name = attribute.get(..end).unwrap_or_default();
            if !valid_identifier(name) {
                return Err(NativeTextFormatError::InvalidFieldAccess(field.to_owned()));
            }
            value = lookup_name(value, name, field)?;
            remainder = attribute.get(end..).unwrap_or_default();
        } else if let Some(indexed) = remainder.strip_prefix('[') {
            let end = indexed
                .find(']')
                .ok_or_else(|| NativeTextFormatError::InvalidFieldAccess(field.to_owned()))?;
            value = lookup_index(value, indexed.get(..end).unwrap_or_default(), field)?;
            remainder = indexed.get(end + 1..).unwrap_or_default();
        } else {
            return Err(NativeTextFormatError::InvalidFieldAccess(field.to_owned()));
        }
    }
    Ok(value)
}

fn lookup_name<'a>(
    value: ValueRef<'a>,
    name: &str,
    field: &str,
) -> Result<ValueRef<'a>, NativeTextFormatError> {
    match value {
        ValueRef::Native(NativeValue::PreservedUnknown { value, .. }) | ValueRef::Json(value) => {
            value
                .as_object()
                .and_then(|object| object.get(name))
                .map(ValueRef::Json)
                .ok_or_else(|| NativeTextFormatError::InvalidFieldAccess(field.to_owned()))
        }
        _ => Err(NativeTextFormatError::InvalidFieldAccess(field.to_owned())),
    }
}

fn lookup_index<'a>(
    value: ValueRef<'a>,
    key: &str,
    field: &str,
) -> Result<ValueRef<'a>, NativeTextFormatError> {
    let invalid = || NativeTextFormatError::InvalidFieldAccess(field.to_owned());
    match value {
        ValueRef::Native(NativeValue::List { values }) => key
            .parse::<usize>()
            .ok()
            .and_then(|index| values.get(index))
            .map(ValueRef::Native)
            .ok_or_else(invalid),
        ValueRef::Native(NativeValue::Primitive {
            value: NativePrimitive::String(value),
        }) => key
            .parse::<usize>()
            .ok()
            .and_then(|index| value.chars().nth(index))
            .map(ValueRef::Character)
            .ok_or_else(invalid),
        ValueRef::Native(NativeValue::PreservedUnknown { value, .. }) | ValueRef::Json(value) => {
            match value {
                Value::Array(values) => key
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| values.get(index)),
                Value::Object(values) => values.get(key),
                _ => None,
            }
            .map(ValueRef::Json)
            .ok_or_else(invalid)
        }
        _ => Err(invalid()),
    }
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic())
        && characters.all(|character| character == '_' || character.is_alphanumeric())
}

enum FormatAtom {
    Text(String),
    Boolean(bool),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

fn format_resolved(
    value: ValueRef<'_>,
    conversion: Option<char>,
    spec: &str,
) -> Result<String, NativeTextFormatError> {
    let atom = match conversion {
        Some('s') => FormatAtom::Text(python_string(value)?),
        Some('r') => FormatAtom::Text(python_repr(value, false)?),
        Some('a') => FormatAtom::Text(python_repr(value, true)?),
        None => default_atom(value)?,
        Some(_) => unreachable!(),
    };
    apply_spec(atom, spec)
}

fn default_atom(value: ValueRef<'_>) -> Result<FormatAtom, NativeTextFormatError> {
    match value {
        ValueRef::Native(NativeValue::Primitive { value }) => Ok(match value {
            NativePrimitive::Null => FormatAtom::Text("None".to_owned()),
            NativePrimitive::Boolean(value) => FormatAtom::Boolean(*value),
            NativePrimitive::Integer(value) => FormatAtom::Signed(*value),
            NativePrimitive::UnsignedInteger(value) => FormatAtom::Unsigned(*value),
            NativePrimitive::Number(value) => FormatAtom::Float(*value),
            NativePrimitive::String(value) => FormatAtom::Text(value.clone()),
        }),
        ValueRef::Json(value) => match value {
            Value::String(value) => Ok(FormatAtom::Text(value.clone())),
            Value::Number(value) => value
                .as_i64()
                .map(FormatAtom::Signed)
                .or_else(|| value.as_u64().map(FormatAtom::Unsigned))
                .or_else(|| value.as_f64().map(FormatAtom::Float))
                .ok_or_else(|| NativeTextFormatError::UnsupportedValue("JSON number".to_owned())),
            Value::Bool(value) => Ok(FormatAtom::Boolean(*value)),
            Value::Null => Ok(FormatAtom::Text("None".to_owned())),
            Value::Array(_) | Value::Object(_) => Ok(FormatAtom::Text(json_repr(value, false)?)),
        },
        value => Ok(FormatAtom::Text(python_string(value)?)),
    }
}

fn python_string(value: ValueRef<'_>) -> Result<String, NativeTextFormatError> {
    match value {
        ValueRef::Character(value) => Ok(value.to_string()),
        ValueRef::Native(NativeValue::Primitive { value }) => Ok(match value {
            NativePrimitive::Null => "None".to_owned(),
            NativePrimitive::Boolean(value) => if *value { "True" } else { "False" }.to_owned(),
            NativePrimitive::Integer(value) => value.to_string(),
            NativePrimitive::UnsignedInteger(value) => value.to_string(),
            NativePrimitive::Number(value) => python_float(*value),
            NativePrimitive::String(value) => value.clone(),
        }),
        ValueRef::Native(NativeValue::Handle { .. }) => Err(
            NativeTextFormatError::UnsupportedValue("process-local handle".to_owned()),
        ),
        value => python_repr(value, false),
    }
}

fn python_repr(value: ValueRef<'_>, ascii_only: bool) -> Result<String, NativeTextFormatError> {
    match value {
        ValueRef::Character(value) => Ok(quoted(&value.to_string(), ascii_only)),
        ValueRef::Native(NativeValue::Primitive { value }) => match value {
            NativePrimitive::String(value) => Ok(quoted(value, ascii_only)),
            NativePrimitive::Null => Ok("None".to_owned()),
            NativePrimitive::Boolean(value) => Ok(if *value { "True" } else { "False" }.to_owned()),
            NativePrimitive::Integer(value) => Ok(value.to_string()),
            NativePrimitive::UnsignedInteger(value) => Ok(value.to_string()),
            NativePrimitive::Number(value) => Ok(python_float(*value)),
        },
        ValueRef::Native(NativeValue::List { values }) => {
            let values = values
                .iter()
                .map(|value| python_repr(ValueRef::Native(value), ascii_only))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", values.join(", ")))
        }
        ValueRef::Native(NativeValue::PreservedUnknown { value, .. }) | ValueRef::Json(value) => {
            json_repr(value, ascii_only)
        }
        ValueRef::Native(NativeValue::Handle { .. }) => Err(
            NativeTextFormatError::UnsupportedValue("process-local handle".to_owned()),
        ),
    }
}

fn json_repr(value: &Value, ascii_only: bool) -> Result<String, NativeTextFormatError> {
    Ok(match value {
        Value::Null => "None".to_owned(),
        Value::Bool(value) => if *value { "True" } else { "False" }.to_owned(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => quoted(value, ascii_only),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| json_repr(value, ascii_only))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| Ok(format!(
                    "{}: {}",
                    quoted(key, ascii_only),
                    json_repr(value, ascii_only)?
                )))
                .collect::<Result<Vec<_>, NativeTextFormatError>>()?
                .join(", ")
        ),
    })
}

fn quoted(value: &str, ascii_only: bool) -> String {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut output = String::new();
    output.push(quote);
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            character if character == quote => {
                output.push('\\');
                output.push(character);
            }
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\x08"),
            '\u{000c}' => output.push_str("\\x0c"),
            character if character.is_control() && u32::from(character) <= 0xff => {
                output.push_str(&format!("\\x{:02x}", u32::from(character)));
            }
            character if ascii_only && !character.is_ascii() => {
                let value = u32::from(character);
                if value <= 0xffff {
                    output.push_str(&format!("\\u{value:04x}"));
                } else {
                    output.push_str(&format!("\\U{value:08x}"));
                }
            }
            character => output.push(character),
        }
    }
    output.push(quote);
    output
}

fn python_float(value: f64) -> String {
    let value = value.to_string();
    if value.contains(['.', 'e', 'E']) || !value.parse::<f64>().is_ok_and(f64::is_finite) {
        value
    } else {
        format!("{value}.0")
    }
}

struct FormatSpec {
    fill: char,
    align: Option<char>,
    sign: Option<char>,
    coerce_negative_zero: bool,
    alternate: bool,
    zero: bool,
    width: Option<usize>,
    grouping: Option<char>,
    precision: Option<usize>,
    kind: Option<char>,
}

impl FormatSpec {
    fn checked(value: &str) -> Result<Self, NativeTextFormatError> {
        let values = value.chars().collect::<Vec<_>>();
        let mut position = 0usize;
        let (fill, align) = if values.get(1).is_some_and(|value| "<>=^".contains(*value)) {
            position = 2;
            (values[0], values.get(1).copied())
        } else if values.first().is_some_and(|value| "<>=^".contains(*value)) {
            position = 1;
            (' ', values.first().copied())
        } else {
            (' ', None)
        };
        let sign = values
            .get(position)
            .copied()
            .filter(|value| "+- ".contains(*value));
        position += usize::from(sign.is_some());
        let coerce_negative_zero = values.get(position) == Some(&'z');
        position += usize::from(coerce_negative_zero);
        let alternate = values.get(position) == Some(&'#');
        position += usize::from(alternate);
        let zero = values.get(position) == Some(&'0');
        position += usize::from(zero);
        let width_start = position;
        while values.get(position).is_some_and(char::is_ascii_digit) {
            position += 1;
        }
        let width = parse_digits(&values[width_start..position])?;
        let grouping = values
            .get(position)
            .copied()
            .filter(|value| matches!(value, ',' | '_'));
        position += usize::from(grouping.is_some());
        let precision = if values.get(position) == Some(&'.') {
            position += 1;
            let start = position;
            while values.get(position).is_some_and(char::is_ascii_digit) {
                position += 1;
            }
            Some(parse_digits(&values[start..position])?.ok_or_else(|| {
                NativeTextFormatError::InvalidTemplate("precision requires digits".to_owned())
            })?)
        } else {
            None
        };
        let kind = values.get(position).copied();
        position += usize::from(kind.is_some());
        if position != values.len() {
            return Err(NativeTextFormatError::InvalidTemplate(
                "unsupported format spec".to_owned(),
            ));
        }
        Ok(Self {
            fill,
            align,
            sign,
            coerce_negative_zero,
            alternate,
            zero,
            width,
            grouping,
            precision,
            kind,
        })
    }
}

fn parse_digits(values: &[char]) -> Result<Option<usize>, NativeTextFormatError> {
    if values.is_empty() {
        return Ok(None);
    }
    values
        .iter()
        .collect::<String>()
        .parse::<usize>()
        .map(Some)
        .map_err(|_| {
            NativeTextFormatError::InvalidTemplate("format number is too large".to_owned())
        })
}

fn apply_spec(value: FormatAtom, spec: &str) -> Result<String, NativeTextFormatError> {
    if spec.is_empty() {
        return Ok(match value {
            FormatAtom::Text(value) => value,
            FormatAtom::Boolean(value) => if value { "True" } else { "False" }.to_owned(),
            FormatAtom::Signed(value) => value.to_string(),
            FormatAtom::Unsigned(value) => value.to_string(),
            FormatAtom::Float(value) => python_float(value),
        });
    }
    let spec = FormatSpec::checked(spec)?;
    let numeric = !matches!(value, FormatAtom::Text(_));
    let mut value = match value {
        FormatAtom::Text(value) => format_text(value, &spec)?,
        FormatAtom::Boolean(value) => format_integer(u64::from(value), false, &spec)?,
        FormatAtom::Signed(value) => format_integer(value.unsigned_abs(), value < 0, &spec)?,
        FormatAtom::Unsigned(value) => format_integer(value, false, &spec)?,
        FormatAtom::Float(value) => format_number(value, &spec)?,
    };
    if let Some(width) = spec.width {
        let length = value.chars().count();
        if length < width {
            let padding = width - length;
            let align = spec.align.unwrap_or(if spec.zero && numeric {
                '='
            } else if numeric {
                '>'
            } else {
                '<'
            });
            let left = match align {
                '>' => padding,
                '^' => padding / 2,
                _ => 0,
            };
            let right = padding - left;
            let fill = if spec.zero && numeric { '0' } else { spec.fill };
            if align == '=' {
                value = pad_numeric_after_prefix(value, fill, padding)?;
            } else {
                value = pad_value(value, fill, left, right)?;
            }
        }
    }
    if value.len() > NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES {
        return Err(NativeTextFormatError::ResultTooLarge {
            maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
        });
    }
    Ok(value)
}

fn format_text(mut value: String, spec: &FormatSpec) -> Result<String, NativeTextFormatError> {
    if spec.sign.is_some()
        || spec.coerce_negative_zero
        || spec.alternate
        || spec.zero
        || spec.grouping.is_some()
        || spec.align == Some('=')
        || !matches!(spec.kind, None | Some('s'))
    {
        return Err(NativeTextFormatError::InvalidTemplate(
            "numeric format applied to text".to_owned(),
        ));
    }
    if let Some(precision) = spec.precision {
        value = value.chars().take(precision).collect();
    }
    Ok(value)
}

fn format_integer(
    magnitude: u64,
    negative: bool,
    spec: &FormatSpec,
) -> Result<String, NativeTextFormatError> {
    if spec.precision.is_some() {
        return Err(NativeTextFormatError::InvalidTemplate(
            "integer precision is unsupported".to_owned(),
        ));
    }
    if spec.coerce_negative_zero {
        return Err(NativeTextFormatError::InvalidTemplate(
            "negative-zero coercion requires a floating-point value".to_owned(),
        ));
    }
    if spec.kind == Some('c') {
        if spec.sign.is_some() || spec.alternate || spec.zero || spec.grouping.is_some() {
            return Err(NativeTextFormatError::InvalidTemplate(
                "unsupported character format option".to_owned(),
            ));
        }
        let codepoint = u32::try_from(magnitude)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| {
                NativeTextFormatError::InvalidTemplate(
                    "character code point is outside the Unicode scalar range".to_owned(),
                )
            })?;
        return Ok(codepoint.to_string());
    }
    let (prefix, mut digits, group_size) = match spec.kind.unwrap_or('d') {
        'd' | 'n' => ("", magnitude.to_string(), 3),
        'b' => (
            if spec.alternate { "0b" } else { "" },
            format!("{magnitude:b}"),
            4,
        ),
        'o' => (
            if spec.alternate { "0o" } else { "" },
            format!("{magnitude:o}"),
            4,
        ),
        'x' => (
            if spec.alternate { "0x" } else { "" },
            format!("{magnitude:x}"),
            4,
        ),
        'X' => (
            if spec.alternate { "0X" } else { "" },
            format!("{magnitude:X}"),
            4,
        ),
        _ => {
            return Err(NativeTextFormatError::InvalidTemplate(
                "unsupported integer format type".to_owned(),
            ));
        }
    };
    if let Some(grouping) = spec.grouping {
        if grouping == ',' && !matches!(spec.kind, None | Some('d' | 'n')) {
            return Err(NativeTextFormatError::InvalidTemplate(
                "comma grouping is unsupported for this integer format type".to_owned(),
            ));
        }
        digits = group_digits(&digits, grouping, group_size);
    }
    Ok(format!(
        "{}{prefix}{digits}",
        sign_prefix(negative, spec.sign)
    ))
}

fn format_number(value: f64, spec: &FormatSpec) -> Result<String, NativeTextFormatError> {
    let mut negative = value.is_sign_negative();
    let precision = spec.precision.unwrap_or(6);
    let magnitude = value.abs();
    let mut formatted = match spec.kind {
        None if spec.precision.is_none() => python_float(magnitude),
        None => format_general(magnitude, precision, false, spec.alternate, true),
        Some('f' | 'F') => format_fixed(magnitude, precision, spec.alternate),
        Some('e') => format_exponent(magnitude, precision, false, spec.alternate),
        Some('E') => format_exponent(magnitude, precision, true, spec.alternate),
        Some('%') => format!(
            "{}%",
            format_fixed(magnitude * 100.0, precision, spec.alternate)
        ),
        Some('g' | 'n') => format_general(magnitude, precision, false, spec.alternate, false),
        Some('G') => format_general(magnitude, precision, true, spec.alternate, false),
        _ => {
            return Err(NativeTextFormatError::InvalidTemplate(
                "unsupported number format type".to_owned(),
            ));
        }
    };
    if spec.coerce_negative_zero && formatted_value_is_zero(&formatted) {
        negative = false;
    }
    if let Some(grouping) = spec.grouping {
        formatted = group_float_integer_part(&formatted, grouping);
    }
    Ok(format!("{}{formatted}", sign_prefix(negative, spec.sign)))
}

fn format_fixed(value: f64, precision: usize, alternate: bool) -> String {
    let mut value = format!("{value:.precision$}");
    if alternate && precision == 0 && !value.contains('.') {
        value.push('.');
    }
    value
}

fn format_exponent(value: f64, precision: usize, uppercase: bool, alternate: bool) -> String {
    let mut value = format!("{value:.precision$e}");
    if alternate && precision == 0 {
        if let Some(exponent) = value.find('e') {
            value.insert(exponent, '.');
        }
    }
    normalize_exponent(value, uppercase)
}

fn format_general(
    value: f64,
    precision: usize,
    uppercase: bool,
    alternate: bool,
    no_type: bool,
) -> String {
    if !value.is_finite() {
        let value = python_float(value);
        return if uppercase {
            value.to_ascii_uppercase()
        } else {
            value
        };
    }
    let precision = precision.max(1);
    if value == 0.0 {
        let mut zero = if alternate {
            format!(
                "{:.precision$}",
                0.0,
                precision = precision.saturating_sub(1)
            )
        } else {
            "0".to_owned()
        };
        if alternate && !zero.contains('.') {
            zero.push('.');
        }
        return zero;
    }
    let exponent = rounded_decimal_exponent(value, precision);
    let positive_cutoff = i32::try_from(precision)
        .unwrap_or(i32::MAX)
        .saturating_sub(i32::from(no_type));
    if exponent < -4 || exponent >= positive_cutoff {
        let mut value = format_exponent(value, precision.saturating_sub(1), uppercase, alternate);
        if !alternate {
            trim_fraction_zeros(&mut value);
        }
        value
    } else {
        let fractional_digits =
            usize::try_from((i32::try_from(precision).unwrap_or(i32::MAX) - exponent - 1).max(0))
                .unwrap_or(0);
        let mut value = format_fixed(value, fractional_digits, alternate);
        if !alternate {
            trim_fraction_zeros(&mut value);
        }
        value
    }
}

fn rounded_decimal_exponent(value: f64, precision: usize) -> i32 {
    let scientific = format!(
        "{value:.precision$e}",
        precision = precision.saturating_sub(1)
    );
    scientific
        .split_once('e')
        .and_then(|(_, exponent)| exponent.parse::<i32>().ok())
        .unwrap_or_else(|| value.log10().floor() as i32)
}

fn normalize_exponent(value: String, uppercase: bool) -> String {
    let marker = value.find('e').or_else(|| value.find('E'));
    let Some(marker) = marker else {
        return if uppercase {
            value.to_ascii_uppercase()
        } else {
            value
        };
    };
    let mantissa = value.get(..marker).unwrap_or_default();
    let exponent = value.get(marker + 1..).unwrap_or_default();
    let parsed = exponent.parse::<i32>().unwrap_or_default();
    let sign = if parsed < 0 { '-' } else { '+' };
    let magnitude = parsed.unsigned_abs();
    let marker = if uppercase { 'E' } else { 'e' };
    format!("{mantissa}{marker}{sign}{magnitude:02}")
}

fn trim_fraction_zeros(value: &mut String) {
    let exponent = value.find(['e', 'E']).unwrap_or(value.len());
    let Some(decimal) = value.get(..exponent).and_then(|prefix| prefix.find('.')) else {
        return;
    };
    let mut end = exponent;
    while end > decimal + 1 && value.as_bytes().get(end - 1) == Some(&b'0') {
        end -= 1;
    }
    if end == decimal + 1 {
        end = decimal;
    }
    value.replace_range(end..exponent, "");
}

fn group_digits(value: &str, separator: char, group_size: usize) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let first_group = characters.len() % group_size;
    let mut output = String::with_capacity(value.len().saturating_add(value.len() / group_size));
    for (index, character) in characters.into_iter().enumerate() {
        if index != 0
            && (index == first_group
                || (index > first_group && (index - first_group).is_multiple_of(group_size)))
        {
            output.push(separator);
        }
        output.push(character);
    }
    output
}

fn group_float_integer_part(value: &str, separator: char) -> String {
    let exponent = value.find(['e', 'E']).unwrap_or(value.len());
    let suffix = value.get(exponent..).unwrap_or_default();
    let mantissa = value.get(..exponent).unwrap_or_default();
    let decimal = mantissa.find('.').unwrap_or(mantissa.len());
    let integer = mantissa.get(..decimal).unwrap_or_default();
    let fraction = mantissa.get(decimal..).unwrap_or_default();
    format!("{}{fraction}{suffix}", group_digits(integer, separator, 3))
}

fn formatted_value_is_zero(value: &str) -> bool {
    let value = value.strip_suffix('%').unwrap_or(value);
    value.parse::<f64>().is_ok_and(|value| value == 0.0)
}

fn pad_numeric_after_prefix(
    value: String,
    fill: char,
    padding: usize,
) -> Result<String, NativeTextFormatError> {
    let sign_length = usize::from(
        value
            .chars()
            .next()
            .is_some_and(|value| matches!(value, '+' | '-' | ' ')),
    );
    let prefix_length = if value.get(sign_length..).is_some_and(|value| {
        value.starts_with("0x")
            || value.starts_with("0X")
            || value.starts_with("0o")
            || value.starts_with("0b")
    }) {
        2
    } else {
        0
    };
    let split = sign_length + prefix_length;
    let (prefix, digits) = value.split_at(split);
    let repeated = repeat_fill(fill, padding, value.len())?;
    Ok(format!("{prefix}{repeated}{digits}"))
}

fn pad_value(
    value: String,
    fill: char,
    left: usize,
    right: usize,
) -> Result<String, NativeTextFormatError> {
    let left = repeat_fill(fill, left, value.len())?;
    let size =
        left.len()
            .checked_add(value.len())
            .ok_or(NativeTextFormatError::ResultTooLarge {
                maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
            })?;
    let right = repeat_fill(fill, right, size)?;
    Ok(format!("{left}{value}{right}"))
}

fn repeat_fill(
    fill: char,
    count: usize,
    existing_bytes: usize,
) -> Result<String, NativeTextFormatError> {
    let repeated_bytes = fill
        .len_utf8()
        .checked_mul(count)
        .and_then(|bytes| bytes.checked_add(existing_bytes))
        .ok_or(NativeTextFormatError::ResultTooLarge {
            maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
        })?;
    if repeated_bytes > NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES {
        return Err(NativeTextFormatError::ResultTooLarge {
            maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
        });
    }
    Ok(fill.to_string().repeat(count))
}

fn sign_prefix(negative: bool, sign: Option<char>) -> &'static str {
    if negative {
        "-"
    } else {
        match sign {
            Some('+') => "+",
            Some(' ') => " ",
            _ => "",
        }
    }
}

fn append_bounded(output: &mut String, value: &str) -> Result<(), NativeTextFormatError> {
    let size =
        output
            .len()
            .checked_add(value.len())
            .ok_or(NativeTextFormatError::ResultTooLarge {
                maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
            })?;
    if size > NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES {
        return Err(NativeTextFormatError::ResultTooLarge {
            maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
        });
    }
    output.push_str(value);
    Ok(())
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), NativeTextFormatError> {
    cancellation
        .check()
        .map_err(|_| NativeTextFormatError::Cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NativeTextRegex, NativeTextRegexFlags};
    use serde::Serialize;
    use sha2::{Digest, Sha256};
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    fn primitive(value: NativePrimitive) -> NativeValue {
        NativeValue::Primitive { value }
    }

    #[test]
    fn named_fields_escapes_conversions_and_nested_specs_are_exact() -> Result<(), Box<dyn Error>> {
        let values = BTreeMap::from([
            (
                "a".to_owned(),
                primitive(NativePrimitive::String("café".to_owned())),
            ),
            ("b".to_owned(), primitive(NativePrimitive::Integer(42))),
            ("c".to_owned(), primitive(NativePrimitive::Integer(6))),
        ]);
        assert_eq!(
            NativeTextFormatter::format(
                "{{{a!r}}} {b:+0{c}x} {a!a}",
                &values,
                &CancellationToken::default(),
            )?,
            "{'café'} +0002a 'caf\\u00e9'"
        );
        Ok(())
    }

    #[test]
    fn structured_item_and_attribute_access_are_checked() -> Result<(), Box<dyn Error>> {
        let values = BTreeMap::from([(
            "a".to_owned(),
            NativeValue::PreservedUnknown {
                type_name: "record".to_owned(),
                value: serde_json::json!({"items": ["first", {"name": "second"}]}),
            },
        )]);
        assert_eq!(
            NativeTextFormatter::format(
                "{a.items[0]} {a.items[1].name}",
                &values,
                &CancellationToken::default(),
            )?,
            "first second"
        );
        Ok(())
    }

    #[test]
    fn python_numeric_format_language_is_source_compatible() -> Result<(), Box<dyn Error>> {
        let values = BTreeMap::from([
            ("a".to_owned(), primitive(NativePrimitive::Integer(42))),
            ("b".to_owned(), primitive(NativePrimitive::Integer(12_345))),
            ("c".to_owned(), primitive(NativePrimitive::Integer(65))),
            ("d".to_owned(), primitive(NativePrimitive::Number(1.0))),
            ("e".to_owned(), primitive(NativePrimitive::Number(12_345.0))),
            ("f".to_owned(), primitive(NativePrimitive::Number(12.34))),
            ("g".to_owned(), primitive(NativePrimitive::Boolean(true))),
        ]);
        assert_eq!(
            NativeTextFormatter::format(
                "{a:=+8d}|{b:,d}|{b:_d}|{c:c}|{d:.2e}|{e:.3g}|{f:.2}|{d:#.0f}|{d:#.3g}|{g:d}|{g:>8}",
                &values,
                &CancellationToken::default(),
            )?,
            "+     42|12,345|12_345|A|1.00e+00|1.23e+04|1.2e+01|1.|1.00|1|       1"
        );
        Ok(())
    }

    #[test]
    fn python_alignment_grouping_alternate_and_type_matrix_is_exact() -> Result<(), Box<dyn Error>>
    {
        let values = BTreeMap::from([
            ("i".to_owned(), primitive(NativePrimitive::Integer(42))),
            ("n".to_owned(), primitive(NativePrimitive::Integer(12_345))),
            ("x".to_owned(), primitive(NativePrimitive::Integer(255))),
            ("z".to_owned(), primitive(NativePrimitive::Number(-0.0))),
            ("v".to_owned(), primitive(NativePrimitive::Number(1_234.5))),
        ]);
        assert_eq!(
            NativeTextFormatter::format(
                "{i:<8d}|{i:^8d}|{i:*>8d}|{i:+d}|{i: d}|{x:#b}|{x:#o}|{x:#x}|{x:#X}|{i:08d}|{x:_b}|{n:,d}|{v:.0f}|{v:.2f}|{v:.1%}|{v:.3E}|{v:.4G}|{v:,.2f}|{z:z.1f}",
                &values,
                &CancellationToken::default(),
            )?,
            "42      |   42   |******42|+42| 42|0b11111111|0o377|0xff|0XFF|00000042|1111_1111|12,345|1234|1234.50|123450.0%|1.234E+03|1234|1,234.50|0.0"
        );
        Ok(())
    }

    #[test]
    fn python_repr_and_result_allocation_bounds_fail_closed_before_repeating()
    -> Result<(), Box<dyn Error>> {
        let values = BTreeMap::from([
            (
                "a".to_owned(),
                primitive(NativePrimitive::String("can't \u{0007} café".to_owned())),
            ),
            ("b".to_owned(), primitive(NativePrimitive::Integer(1))),
        ]);
        assert_eq!(
            NativeTextFormatter::format("{a!r}|{a!a}", &values, &CancellationToken::default(),)?,
            "\"can't \\x07 café\"|\"can't \\x07 caf\\u00e9\""
        );
        assert_eq!(
            NativeTextFormatter::format("{b:😀>999999999}", &values, &CancellationToken::default(),),
            Err(NativeTextFormatError::ResultTooLarge {
                maximum_bytes: NATIVE_TEXT_FORMAT_MAX_RESULT_BYTES,
            })
        );
        Ok(())
    }

    #[test]
    fn invalid_missing_and_cancelled_formats_fail_closed() {
        let values = BTreeMap::new();
        assert!(matches!(
            NativeTextFormatter::format("{a}", &values, &CancellationToken::default()),
            Err(NativeTextFormatError::MissingField(_))
        ));
        assert!(matches!(
            NativeTextFormatter::format("{", &values, &CancellationToken::default()),
            Err(NativeTextFormatError::InvalidTemplate(_))
        ));
        let cancellation = CancellationToken::default();
        assert!(cancellation.cancel());
        assert_eq!(
            NativeTextFormatter::format("text", &values, &cancellation),
            Err(NativeTextFormatError::Cancelled)
        );
    }

    #[derive(Serialize)]
    struct TextTransformValidationArtifact {
        validation_id: &'static str,
        scope: &'static str,
        source_path: &'static str,
        source_sha256: &'static str,
        fixture_sha256: String,
        environment: TextTransformValidationEnvironment,
        cases: Vec<serde_json::Value>,
        passed: usize,
        failed: usize,
        skipped: usize,
    }

    #[derive(Serialize)]
    struct TextTransformValidationEnvironment {
        operating_system: &'static str,
        architecture: &'static str,
        backend_identity: &'static str,
    }

    #[test]
    fn val_node_002_text_transform_source_oracle() -> Result<(), Box<dyn Error>> {
        let cancellation = CancellationToken::default();
        let regex = NativeTextRegex::checked(r"(?P<word>a)(b)?", NativeTextRegexFlags::default())?;
        let cases = vec![
            serde_json::json!({
                "case_id": "regex-python-octal-and-unmatched-group",
                "actual": regex.replace("a", "\\0-\\123-\\08-\\g<0>-\\g<word>-\\2", 0, &cancellation)?,
                "expected": "\u{0000}-S-\u{0000}8-a-a-",
                "passed": true,
            }),
            serde_json::json!({
                "case_id": "regex-python-zero-width-duplicate-order",
                "actual": NativeTextRegex::checked(r"x*", NativeTextRegexFlags::default())?
                    .replace("abxd", "-", 0, &cancellation)?,
                "expected": "-a-b--d-",
                "passed": true,
            }),
            serde_json::json!({
                "case_id": "regex-invalid-reference-fails-closed",
                "passed": matches!(
                    regex.replace("a", r"\3", 0, &cancellation),
                    Err(crate::NativeTextRegexError::InvalidReplacement(_))
                ),
            }),
            serde_json::json!({
                "case_id": "format-python-numeric-mini-language",
                "actual": NativeTextFormatter::format(
                    "{a:=+8d}|{b:,d}|{c:c}|{d:.2e}|{e:.3g}|{f:.2}|{g:d}",
                    &BTreeMap::from([
                        ("a".to_owned(), primitive(NativePrimitive::Integer(42))),
                        ("b".to_owned(), primitive(NativePrimitive::Integer(12_345))),
                        ("c".to_owned(), primitive(NativePrimitive::Integer(65))),
                        ("d".to_owned(), primitive(NativePrimitive::Number(1.0))),
                        ("e".to_owned(), primitive(NativePrimitive::Number(12_345.0))),
                        ("f".to_owned(), primitive(NativePrimitive::Number(12.34))),
                        ("g".to_owned(), primitive(NativePrimitive::Boolean(true))),
                    ]),
                    &cancellation,
                )?,
                "expected": "+     42|12,345|A|1.00e+00|1.23e+04|1.2e+01|1",
                "passed": true,
            }),
            serde_json::json!({
                "case_id": "format-preallocation-bound",
                "passed": matches!(
                    NativeTextFormatter::format(
                        "{a:😀>999999999}",
                        &BTreeMap::from([("a".to_owned(), primitive(NativePrimitive::Integer(1)))]),
                        &cancellation,
                    ),
                    Err(NativeTextFormatError::ResultTooLarge { .. })
                ),
            }),
        ];
        assert!(
            cases.iter().all(|case| {
                case.get("passed") == Some(&serde_json::Value::Bool(true))
                    && case
                        .get("actual")
                        .zip(case.get("expected"))
                        .is_none_or(|(actual, expected)| actual == expected)
            }),
            "source-oracle mismatch: {cases:#?}"
        );
        let fixture_bytes = serde_json::to_vec(&cases)?;
        let fixture_sha256 = format!("{:x}", Sha256::digest(&fixture_bytes));
        let artifact = TextTransformValidationArtifact {
            validation_id: "VAL-NODE-002",
            scope: "native text transform source oracle",
            source_path: "projects/comfy/ComfyUI/comfy_extras/nodes_string.py",
            source_sha256: "bb01963178f28efc6e3a9578aecb89f53f8265e89c0518da92ec21c4955e85f1",
            fixture_sha256,
            environment: TextTransformValidationEnvironment {
                operating_system: std::env::consts::OS,
                architecture: std::env::consts::ARCH,
                backend_identity: "native-rust-cpu-source-oracle",
            },
            passed: cases.len(),
            failed: 0,
            skipped: 0,
            cases,
        };
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("workspace root is unavailable")?;
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .map(|target| {
                if target.is_absolute() {
                    target
                } else {
                    workspace_root.join(target)
                }
            })
            .unwrap_or_else(|| workspace_root.join("target"));
        let artifact_directory = target.join("comfy-parity");
        fs::create_dir_all(&artifact_directory)?;
        let mut artifact_bytes = serde_json::to_vec_pretty(&artifact)?;
        artifact_bytes.push(b'\n');
        fs::write(artifact_directory.join("val-node-002.json"), artifact_bytes)?;
        Ok(())
    }
}
