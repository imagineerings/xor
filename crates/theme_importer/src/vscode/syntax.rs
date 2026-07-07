use indexmap::IndexMap;
use serde::Deserialize;
use strum::EnumIter;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum VsCodeTokenScope {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct VsCodeTokenColor {
    pub name: Option<String>,
    pub scope: Option<VsCodeTokenScope>,
    pub settings: VsCodeTokenColorSettings,
}

#[derive(Debug, Deserialize)]
pub struct VsCodeTokenColorSettings {
    pub foreground: Option<String>,
    pub background: Option<String>,
    #[serde(rename = "fontStyle")]
    pub font_style: Option<String>,
}

#[derive(Debug, PartialEq, Copy, Clone, EnumIter)]
pub enum SimSyntaxToken {
    Attribute,
    Boolean,
    Comment,
    CommentDoc,
    Constant,
    Constructor,
    Embedded,
    Emphasis,
    EmphasisStrong,
    Enum,
    Function,
    Hint,
    Keyword,
    Label,
    LinkText,
    LinkUri,
    Number,
    Operator,
    Predictive,
    Preproc,
    Primary,
    Property,
    Punctuation,
    PunctuationBracket,
    PunctuationDelimiter,
    PunctuationListMarker,
    PunctuationSpecial,
    String,
    StringEscape,
    StringRegex,
    StringSpecial,
    StringSpecialSymbol,
    Tag,
    TextLiteral,
    Title,
    Type,
    Variable,
    VariableSpecial,
    Variant,
}

impl std::fmt::Display for SimSyntaxToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SimSyntaxToken::Attribute => "attribute",
                SimSyntaxToken::Boolean => "boolean",
                SimSyntaxToken::Comment => "comment",
                SimSyntaxToken::CommentDoc => "comment.doc",
                SimSyntaxToken::Constant => "constant",
                SimSyntaxToken::Constructor => "constructor",
                SimSyntaxToken::Embedded => "embedded",
                SimSyntaxToken::Emphasis => "emphasis",
                SimSyntaxToken::EmphasisStrong => "emphasis.strong",
                SimSyntaxToken::Enum => "enum",
                SimSyntaxToken::Function => "function",
                SimSyntaxToken::Hint => "hint",
                SimSyntaxToken::Keyword => "keyword",
                SimSyntaxToken::Label => "label",
                SimSyntaxToken::LinkText => "link_text",
                SimSyntaxToken::LinkUri => "link_uri",
                SimSyntaxToken::Number => "number",
                SimSyntaxToken::Operator => "operator",
                SimSyntaxToken::Predictive => "predictive",
                SimSyntaxToken::Preproc => "preproc",
                SimSyntaxToken::Primary => "primary",
                SimSyntaxToken::Property => "property",
                SimSyntaxToken::Punctuation => "punctuation",
                SimSyntaxToken::PunctuationBracket => "punctuation.bracket",
                SimSyntaxToken::PunctuationDelimiter => "punctuation.delimiter",
                SimSyntaxToken::PunctuationListMarker => "punctuation.list_marker",
                SimSyntaxToken::PunctuationSpecial => "punctuation.special",
                SimSyntaxToken::String => "string",
                SimSyntaxToken::StringEscape => "string.escape",
                SimSyntaxToken::StringRegex => "string.regex",
                SimSyntaxToken::StringSpecial => "string.special",
                SimSyntaxToken::StringSpecialSymbol => "string.special.symbol",
                SimSyntaxToken::Tag => "tag",
                SimSyntaxToken::TextLiteral => "text.literal",
                SimSyntaxToken::Title => "title",
                SimSyntaxToken::Type => "type",
                SimSyntaxToken::Variable => "variable",
                SimSyntaxToken::VariableSpecial => "variable.special",
                SimSyntaxToken::Variant => "variant",
            }
        )
    }
}

impl SimSyntaxToken {
    pub fn find_best_token_color_match<'a>(
        &self,
        token_colors: &'a [VsCodeTokenColor],
    ) -> Option<&'a VsCodeTokenColor> {
        let mut ranked_matches = IndexMap::new();

        for (ix, token_color) in token_colors.iter().enumerate() {
            if token_color.settings.foreground.is_none() {
                continue;
            }

            let Some(rank) = self.rank_match(token_color) else {
                continue;
            };

            if rank > 0 {
                ranked_matches.insert(ix, rank);
            }
        }

        ranked_matches
            .into_iter()
            .max_by_key(|(_, rank)| *rank)
            .map(|(ix, _)| &token_colors[ix])
    }

    fn rank_match(&self, token_color: &VsCodeTokenColor) -> Option<u32> {
        let candidate_scopes = match token_color.scope.as_ref()? {
            VsCodeTokenScope::One(scope) => vec![scope],
            VsCodeTokenScope::Many(scopes) => scopes.iter().collect(),
        }
        .iter()
        .flat_map(|scope| scope.split(',').map(|s| s.trim()))
        .collect::<Vec<_>>();

        let scopes_to_match = self.to_vscode();
        let number_of_scopes_to_match = scopes_to_match.len();

        let mut matches = 0;

        for (ix, scope) in scopes_to_match.into_iter().enumerate() {
            // Assign each entry a weight that is inversely proportional to its
            // position in the list.
            //
            // Entries towards the front are weighted higher than those towards the end.
            let weight = (number_of_scopes_to_match - ix) as u32;

            if candidate_scopes.contains(&scope) {
                matches += 1 + weight;
            }
        }

        Some(matches)
    }

    pub fn fallbacks(&self) -> &[Self] {
        match self {
            SimSyntaxToken::CommentDoc => &[SimSyntaxToken::Comment],
            SimSyntaxToken::Number => &[SimSyntaxToken::Constant],
            SimSyntaxToken::VariableSpecial => &[SimSyntaxToken::Variable],
            SimSyntaxToken::PunctuationBracket
            | SimSyntaxToken::PunctuationDelimiter
            | SimSyntaxToken::PunctuationListMarker
            | SimSyntaxToken::PunctuationSpecial => &[SimSyntaxToken::Punctuation],
            SimSyntaxToken::StringEscape
            | SimSyntaxToken::StringRegex
            | SimSyntaxToken::StringSpecial
            | SimSyntaxToken::StringSpecialSymbol => &[SimSyntaxToken::String],
            _ => &[],
        }
    }

    fn to_vscode(self) -> Vec<&'static str> {
        match self {
            SimSyntaxToken::Attribute => vec!["entity.other.attribute-name"],
            SimSyntaxToken::Boolean => vec!["constant.language"],
            SimSyntaxToken::Comment => vec!["comment"],
            SimSyntaxToken::CommentDoc => vec!["comment.block.documentation"],
            SimSyntaxToken::Constant => {
                vec!["constant", "constant.language", "constant.character"]
            }
            SimSyntaxToken::Constructor => {
                vec![
                    "entity.name.tag",
                    "entity.name.function.definition.special.constructor",
                ]
            }
            SimSyntaxToken::Embedded => vec!["meta.embedded"],
            SimSyntaxToken::Emphasis => vec!["markup.italic"],
            SimSyntaxToken::EmphasisStrong => vec![
                "markup.bold",
                "markup.italic markup.bold",
                "markup.bold markup.italic",
            ],
            SimSyntaxToken::Enum => vec!["support.type.enum"],
            SimSyntaxToken::Function => vec![
                "entity.function",
                "entity.name.function",
                "variable.function",
            ],
            SimSyntaxToken::Hint => vec![],
            SimSyntaxToken::Keyword => vec![
                "keyword",
                "keyword.other.fn.rust",
                "keyword.control",
                "keyword.control.fun",
                "keyword.control.class",
                "punctuation.accessor",
                "entity.name.tag",
            ],
            SimSyntaxToken::Label => vec![
                "label",
                "entity.name",
                "entity.name.import",
                "entity.name.package",
            ],
            SimSyntaxToken::LinkText => vec!["markup.underline.link", "string.other.link"],
            SimSyntaxToken::LinkUri => vec!["markup.underline.link", "string.other.link"],
            SimSyntaxToken::Number => vec!["constant.numeric", "number"],
            SimSyntaxToken::Operator => vec!["operator", "keyword.operator"],
            SimSyntaxToken::Predictive => vec![],
            SimSyntaxToken::Preproc => vec![
                "preproc",
                "meta.preprocessor",
                "punctuation.definition.preprocessor",
            ],
            SimSyntaxToken::Primary => vec![],
            SimSyntaxToken::Property => vec![
                "variable.member",
                "support.type.property-name",
                "variable.object.property",
                "variable.other.field",
            ],
            SimSyntaxToken::Punctuation => vec![
                "punctuation",
                "punctuation.section",
                "punctuation.accessor",
                "punctuation.separator",
                "punctuation.definition.tag",
            ],
            SimSyntaxToken::PunctuationBracket => vec![
                "punctuation.bracket",
                "punctuation.definition.tag.begin",
                "punctuation.definition.tag.end",
            ],
            SimSyntaxToken::PunctuationDelimiter => vec![
                "punctuation.delimiter",
                "punctuation.separator",
                "punctuation.terminator",
            ],
            SimSyntaxToken::PunctuationListMarker => {
                vec!["markup.list punctuation.definition.list.begin"]
            }
            SimSyntaxToken::PunctuationSpecial => vec!["punctuation.special"],
            SimSyntaxToken::String => vec!["string"],
            SimSyntaxToken::StringEscape => {
                vec!["string.escape", "constant.character", "constant.other"]
            }
            SimSyntaxToken::StringRegex => vec!["string.regex"],
            SimSyntaxToken::StringSpecial => vec!["string.special", "constant.other.symbol"],
            SimSyntaxToken::StringSpecialSymbol => {
                vec!["string.special.symbol", "constant.other.symbol"]
            }
            SimSyntaxToken::Tag => vec!["tag", "entity.name.tag", "meta.tag.sgml"],
            SimSyntaxToken::TextLiteral => vec!["text.literal", "string"],
            SimSyntaxToken::Title => vec!["title", "entity.name"],
            SimSyntaxToken::Type => vec![
                "entity.name.type",
                "entity.name.type.primitive",
                "entity.name.type.numeric",
                "keyword.type",
                "support.type",
                "support.type.primitive",
                "support.class",
            ],
            SimSyntaxToken::Variable => vec![
                "variable",
                "variable.language",
                "variable.member",
                "variable.parameter",
                "variable.parameter.function-call",
            ],
            SimSyntaxToken::VariableSpecial => vec![
                "variable.special",
                "variable.member",
                "variable.annotation",
                "variable.language",
            ],
            SimSyntaxToken::Variant => vec!["variant"],
        }
    }
}
