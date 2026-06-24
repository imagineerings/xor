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
pub enum BaymaxSyntaxToken {
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

impl std::fmt::Display for BaymaxSyntaxToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BaymaxSyntaxToken::Attribute => "attribute",
                BaymaxSyntaxToken::Boolean => "boolean",
                BaymaxSyntaxToken::Comment => "comment",
                BaymaxSyntaxToken::CommentDoc => "comment.doc",
                BaymaxSyntaxToken::Constant => "constant",
                BaymaxSyntaxToken::Constructor => "constructor",
                BaymaxSyntaxToken::Embedded => "embedded",
                BaymaxSyntaxToken::Emphasis => "emphasis",
                BaymaxSyntaxToken::EmphasisStrong => "emphasis.strong",
                BaymaxSyntaxToken::Enum => "enum",
                BaymaxSyntaxToken::Function => "function",
                BaymaxSyntaxToken::Hint => "hint",
                BaymaxSyntaxToken::Keyword => "keyword",
                BaymaxSyntaxToken::Label => "label",
                BaymaxSyntaxToken::LinkText => "link_text",
                BaymaxSyntaxToken::LinkUri => "link_uri",
                BaymaxSyntaxToken::Number => "number",
                BaymaxSyntaxToken::Operator => "operator",
                BaymaxSyntaxToken::Predictive => "predictive",
                BaymaxSyntaxToken::Preproc => "preproc",
                BaymaxSyntaxToken::Primary => "primary",
                BaymaxSyntaxToken::Property => "property",
                BaymaxSyntaxToken::Punctuation => "punctuation",
                BaymaxSyntaxToken::PunctuationBracket => "punctuation.bracket",
                BaymaxSyntaxToken::PunctuationDelimiter => "punctuation.delimiter",
                BaymaxSyntaxToken::PunctuationListMarker => "punctuation.list_marker",
                BaymaxSyntaxToken::PunctuationSpecial => "punctuation.special",
                BaymaxSyntaxToken::String => "string",
                BaymaxSyntaxToken::StringEscape => "string.escape",
                BaymaxSyntaxToken::StringRegex => "string.regex",
                BaymaxSyntaxToken::StringSpecial => "string.special",
                BaymaxSyntaxToken::StringSpecialSymbol => "string.special.symbol",
                BaymaxSyntaxToken::Tag => "tag",
                BaymaxSyntaxToken::TextLiteral => "text.literal",
                BaymaxSyntaxToken::Title => "title",
                BaymaxSyntaxToken::Type => "type",
                BaymaxSyntaxToken::Variable => "variable",
                BaymaxSyntaxToken::VariableSpecial => "variable.special",
                BaymaxSyntaxToken::Variant => "variant",
            }
        )
    }
}

impl BaymaxSyntaxToken {
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
            BaymaxSyntaxToken::CommentDoc => &[BaymaxSyntaxToken::Comment],
            BaymaxSyntaxToken::Number => &[BaymaxSyntaxToken::Constant],
            BaymaxSyntaxToken::VariableSpecial => &[BaymaxSyntaxToken::Variable],
            BaymaxSyntaxToken::PunctuationBracket
            | BaymaxSyntaxToken::PunctuationDelimiter
            | BaymaxSyntaxToken::PunctuationListMarker
            | BaymaxSyntaxToken::PunctuationSpecial => &[BaymaxSyntaxToken::Punctuation],
            BaymaxSyntaxToken::StringEscape
            | BaymaxSyntaxToken::StringRegex
            | BaymaxSyntaxToken::StringSpecial
            | BaymaxSyntaxToken::StringSpecialSymbol => &[BaymaxSyntaxToken::String],
            _ => &[],
        }
    }

    fn to_vscode(self) -> Vec<&'static str> {
        match self {
            BaymaxSyntaxToken::Attribute => vec!["entity.other.attribute-name"],
            BaymaxSyntaxToken::Boolean => vec!["constant.language"],
            BaymaxSyntaxToken::Comment => vec!["comment"],
            BaymaxSyntaxToken::CommentDoc => vec!["comment.block.documentation"],
            BaymaxSyntaxToken::Constant => vec!["constant", "constant.language", "constant.character"],
            BaymaxSyntaxToken::Constructor => {
                vec![
                    "entity.name.tag",
                    "entity.name.function.definition.special.constructor",
                ]
            }
            BaymaxSyntaxToken::Embedded => vec!["meta.embedded"],
            BaymaxSyntaxToken::Emphasis => vec!["markup.italic"],
            BaymaxSyntaxToken::EmphasisStrong => vec![
                "markup.bold",
                "markup.italic markup.bold",
                "markup.bold markup.italic",
            ],
            BaymaxSyntaxToken::Enum => vec!["support.type.enum"],
            BaymaxSyntaxToken::Function => vec![
                "entity.function",
                "entity.name.function",
                "variable.function",
            ],
            BaymaxSyntaxToken::Hint => vec![],
            BaymaxSyntaxToken::Keyword => vec![
                "keyword",
                "keyword.other.fn.rust",
                "keyword.control",
                "keyword.control.fun",
                "keyword.control.class",
                "punctuation.accessor",
                "entity.name.tag",
            ],
            BaymaxSyntaxToken::Label => vec![
                "label",
                "entity.name",
                "entity.name.import",
                "entity.name.package",
            ],
            BaymaxSyntaxToken::LinkText => vec!["markup.underline.link", "string.other.link"],
            BaymaxSyntaxToken::LinkUri => vec!["markup.underline.link", "string.other.link"],
            BaymaxSyntaxToken::Number => vec!["constant.numeric", "number"],
            BaymaxSyntaxToken::Operator => vec!["operator", "keyword.operator"],
            BaymaxSyntaxToken::Predictive => vec![],
            BaymaxSyntaxToken::Preproc => vec![
                "preproc",
                "meta.preprocessor",
                "punctuation.definition.preprocessor",
            ],
            BaymaxSyntaxToken::Primary => vec![],
            BaymaxSyntaxToken::Property => vec![
                "variable.member",
                "support.type.property-name",
                "variable.object.property",
                "variable.other.field",
            ],
            BaymaxSyntaxToken::Punctuation => vec![
                "punctuation",
                "punctuation.section",
                "punctuation.accessor",
                "punctuation.separator",
                "punctuation.definition.tag",
            ],
            BaymaxSyntaxToken::PunctuationBracket => vec![
                "punctuation.bracket",
                "punctuation.definition.tag.begin",
                "punctuation.definition.tag.end",
            ],
            BaymaxSyntaxToken::PunctuationDelimiter => vec![
                "punctuation.delimiter",
                "punctuation.separator",
                "punctuation.terminator",
            ],
            BaymaxSyntaxToken::PunctuationListMarker => {
                vec!["markup.list punctuation.definition.list.begin"]
            }
            BaymaxSyntaxToken::PunctuationSpecial => vec!["punctuation.special"],
            BaymaxSyntaxToken::String => vec!["string"],
            BaymaxSyntaxToken::StringEscape => {
                vec!["string.escape", "constant.character", "constant.other"]
            }
            BaymaxSyntaxToken::StringRegex => vec!["string.regex"],
            BaymaxSyntaxToken::StringSpecial => vec!["string.special", "constant.other.symbol"],
            BaymaxSyntaxToken::StringSpecialSymbol => {
                vec!["string.special.symbol", "constant.other.symbol"]
            }
            BaymaxSyntaxToken::Tag => vec!["tag", "entity.name.tag", "meta.tag.sgml"],
            BaymaxSyntaxToken::TextLiteral => vec!["text.literal", "string"],
            BaymaxSyntaxToken::Title => vec!["title", "entity.name"],
            BaymaxSyntaxToken::Type => vec![
                "entity.name.type",
                "entity.name.type.primitive",
                "entity.name.type.numeric",
                "keyword.type",
                "support.type",
                "support.type.primitive",
                "support.class",
            ],
            BaymaxSyntaxToken::Variable => vec![
                "variable",
                "variable.language",
                "variable.member",
                "variable.parameter",
                "variable.parameter.function-call",
            ],
            BaymaxSyntaxToken::VariableSpecial => vec![
                "variable.special",
                "variable.member",
                "variable.annotation",
                "variable.language",
            ],
            BaymaxSyntaxToken::Variant => vec!["variant"],
        }
    }
}
