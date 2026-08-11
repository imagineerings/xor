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
            NativePrimitive::Boolean(value) => {
                FormatAtom::Text(if *value { "True" } else { "False" }.to_owned())
            }
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
            Value::Bool(value) => Ok(FormatAtom::Text(
                if *value { "True" } else { "False" }.to_owned(),
            )),
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
    let mut output = String::from("'");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\'' => output.push_str("\\'"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
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
    output.push('\'');
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
    alternate: bool,
    zero: bool,
    width: Option<usize>,
    precision: Option<usize>,
    kind: Option<char>,
}

impl FormatSpec {
    fn checked(value: &str) -> Result<Self, NativeTextFormatError> {
        let values = value.chars().collect::<Vec<_>>();
        let mut position = 0usize;
        let (fill, align) = if values.get(1).is_some_and(|value| "<>^".contains(*value)) {
            position = 2;
            (values[0], values.get(1).copied())
        } else if values.first().is_some_and(|value| "<>^".contains(*value)) {
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
        let alternate = values.get(position) == Some(&'#');
        position += usize::from(alternate);
        let zero = values.get(position) == Some(&'0');
        position += usize::from(zero);
        let width_start = position;
        while values.get(position).is_some_and(char::is_ascii_digit) {
            position += 1;
        }
        let width = parse_digits(&values[width_start..position])?;
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
            alternate,
            zero,
            width,
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
            FormatAtom::Signed(value) => value.to_string(),
            FormatAtom::Unsigned(value) => value.to_string(),
            FormatAtom::Float(value) => python_float(value),
        });
    }
    let spec = FormatSpec::checked(spec)?;
    let numeric = !matches!(value, FormatAtom::Text(_));
    let mut value = match value {
        FormatAtom::Text(value) => format_text(value, &spec)?,
        FormatAtom::Signed(value) => format_integer(value.unsigned_abs(), value < 0, &spec)?,
        FormatAtom::Unsigned(value) => format_integer(value, false, &spec)?,
        FormatAtom::Float(value) => format_number(value, &spec)?,
    };
    if let Some(width) = spec.width {
        let length = value.chars().count();
        if length < width {
            let padding = width - length;
            let align = spec.align.unwrap_or(if numeric { '>' } else { '<' });
            let left = match align {
                '>' => padding,
                '^' => padding / 2,
                _ => 0,
            };
            let right = padding - left;
            let fill = if spec.zero && numeric { '0' } else { spec.fill };
            if fill == '0' && numeric && left != 0 {
                value = zero_pad_numeric(value, left);
                value.push_str(&fill.to_string().repeat(right));
            } else {
                value = format!(
                    "{}{}{}",
                    fill.to_string().repeat(left),
                    value,
                    fill.to_string().repeat(right)
                );
            }
        }
    }
    Ok(value)
}

fn format_text(mut value: String, spec: &FormatSpec) -> Result<String, NativeTextFormatError> {
    if spec.sign.is_some() || spec.alternate || spec.zero || !matches!(spec.kind, None | Some('s'))
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
    let (prefix, digits) = match spec.kind.unwrap_or('d') {
        'd' | 'n' => ("", magnitude.to_string()),
        'b' => (
            if spec.alternate { "0b" } else { "" },
            format!("{magnitude:b}"),
        ),
        'o' => (
            if spec.alternate { "0o" } else { "" },
            format!("{magnitude:o}"),
        ),
        'x' => (
            if spec.alternate { "0x" } else { "" },
            format!("{magnitude:x}"),
        ),
        'X' => (
            if spec.alternate { "0X" } else { "" },
            format!("{magnitude:X}"),
        ),
        _ => {
            return Err(NativeTextFormatError::InvalidTemplate(
                "unsupported integer format type".to_owned(),
            ));
        }
    };
    Ok(format!(
        "{}{prefix}{digits}",
        sign_prefix(negative, spec.sign)
    ))
}

fn format_number(value: f64, spec: &FormatSpec) -> Result<String, NativeTextFormatError> {
    let negative = value.is_sign_negative();
    let precision = spec.precision.unwrap_or(6);
    let magnitude = value.abs();
    let value = match spec.kind {
        None => python_float(magnitude),
        Some('f' | 'F') => format!("{magnitude:.precision$}"),
        Some('e') => format!("{magnitude:.precision$e}"),
        Some('E') => format!("{magnitude:.precision$E}"),
        Some('%') => format!("{:.precision$}%", magnitude * 100.0),
        Some('g' | 'G' | 'n') => python_float(magnitude),
        _ => {
            return Err(NativeTextFormatError::InvalidTemplate(
                "unsupported number format type".to_owned(),
            ));
        }
    };
    Ok(format!("{}{value}", sign_prefix(negative, spec.sign)))
}

fn zero_pad_numeric(value: String, padding: usize) -> String {
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
    format!("{prefix}{}{digits}", "0".repeat(padding))
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
    use std::error::Error;

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
}
