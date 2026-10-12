//! A typed lookup IR for Adobe feature syntax.
//!
//! [`parse_fea`] turns feature source into an ordered list of [`Statement`]s,
//! with typed rules for the GSUB and GPOS lookup types FontCarver authors
//! visually (GSUB 1/2/3/4/5/6/8 and GPOS 1/2/3/4/5/6/7/8) plus glyph classes,
//! mark classes, language systems, lookup flags, script/language scopes, named
//! lookups and `aalt` feature references. Anything the IR does not model is
//! kept as [`Statement::Verbatim`] holding the exact original text, so no user
//! source is ever dropped.
//!
//! [`write_fea`] renders the IR back to canonical feature syntax. The pair is a
//! semantic round trip: parsing the rendered text yields an equal IR.

mod parse;
mod write;

pub use parse::{parse_fea, ParseError};
pub use write::write_fea;

/// Which table a typed rule compiles into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LookupTable {
    /// Glyph substitution.
    Gsub,
    /// Glyph positioning.
    Gpos,
}

/// The table and lookup-type number a rule belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LookupType {
    /// GSUB or GPOS.
    pub table: LookupTable,
    /// OpenType lookup type number, 1 through 8.
    pub number: u8,
}

impl LookupType {
    /// A GSUB lookup type.
    pub const fn gsub(number: u8) -> Self {
        Self {
            table: LookupTable::Gsub,
            number,
        }
    }

    /// A GPOS lookup type.
    pub const fn gpos(number: u8) -> Self {
        Self {
            table: LookupTable::Gpos,
            number,
        }
    }
}

impl std::fmt::Display for LookupType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let table = match self.table {
            LookupTable::Gsub => "GSUB",
            LookupTable::Gpos => "GPOS",
        };
        write!(formatter, "{table} {}", self.number)
    }
}

/// One member of a glyph class literal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassItem {
    /// A literal glyph name.
    Glyph(String),
    /// An inclusive glyph-name range.
    Range(String, String),
    /// A glyph index in the original font.
    Cid(u32),
    /// An inclusive CID range.
    CidRange(u32, u32),
    /// A reference to another named class.
    Named(String),
    /// A hyphenated token that needs the font's glyph names to disambiguate.
    ///
    /// Parsing without a glyph order cannot tell `c-f` (a range) from a glyph
    /// literally called `c-f`; the original text is kept unchanged.
    Unresolved(String),
}

/// A glyph operand in a rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlyphSet {
    /// One glyph name.
    Glyph(String),
    /// One glyph index in the original font.
    Cid(u32),
    /// A reference to a named class.
    Named(String),
    /// An inline class literal.
    Literal(Vec<ClassItem>),
    /// The explicit empty slot `NULL`.
    Null,
}

/// A GPOS value record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueRecord {
    /// A bare advance adjustment along the writing direction.
    Advance(i32),
    /// The four-number placement and advance form.
    Placement {
        /// X placement.
        x_placement: i32,
        /// Y placement.
        y_placement: i32,
        /// X advance.
        x_advance: i32,
        /// Y advance.
        y_advance: i32,
    },
    /// `<NULL>`.
    Null,
    /// A named record, a device-table record or a variable metric, kept as its
    /// exact original text.
    Other(String),
}

/// A GPOS attachment anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorPoint {
    /// Plain design-space coordinates.
    At {
        /// X coordinate.
        x: i32,
        /// Y coordinate.
        y: i32,
    },
    /// Coordinates bound to a contour point index.
    Contour {
        /// X coordinate.
        x: i32,
        /// Y coordinate.
        y: i32,
        /// Contour point index.
        point: u32,
    },
    /// `<anchor NULL>`, the explicitly absent attachment.
    Null,
    /// A named anchor or a variable anchor, kept as its exact original text.
    Other(String),
}

/// One mark class attached at an anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkAttachment {
    /// The attachment point on the base, mark or ligature component.
    pub anchor: AnchorPoint,
    /// The mark class attached there; absent for an empty ligature component.
    pub class: Option<String>,
}

/// One marked input position of a contextual rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextItem {
    /// Glyphs matched at this position.
    pub glyphs: GlyphSet,
    /// Named lookups invoked at this position, in order.
    pub lookups: Vec<String>,
    /// A positioning adjustment applied at this position.
    pub value: Option<ValueRecord>,
}

/// How a pair-positioning rule spells its value records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairValues {
    /// `pos A B <value>`: one record after both glyphs, adjusting the first.
    AfterPair(ValueRecord),
    /// `pos A <value> B`: a record after the first glyph only.
    AfterFirst(ValueRecord),
    /// `pos A <value> B <value>`: a record after each glyph.
    Both(ValueRecord, ValueRecord),
}

/// One context pattern of an `ignore` rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnorePattern {
    /// Glyphs before the input sequence.
    pub backtrack: Vec<GlyphSet>,
    /// The marked input sequence.
    pub input: Vec<GlyphSet>,
    /// Glyphs after the input sequence.
    pub lookahead: Vec<GlyphSet>,
}

/// A typed substitution or positioning rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// GSUB 1: one input glyph or class becomes one output.
    SingleSub {
        /// Input glyphs.
        target: GlyphSet,
        /// Replacement glyphs.
        replacement: GlyphSet,
    },
    /// GSUB 2: one input glyph becomes a sequence.
    MultipleSub {
        /// Input glyph.
        target: GlyphSet,
        /// Replacement sequence.
        replacement: Vec<GlyphSet>,
    },
    /// GSUB 3: one input glyph offers a set of alternates.
    AlternateSub {
        /// Input glyph.
        target: GlyphSet,
        /// Offered alternates.
        alternates: GlyphSet,
    },
    /// GSUB 4: a sequence becomes one glyph.
    LigatureSub {
        /// Input sequence.
        target: Vec<GlyphSet>,
        /// Replacement glyph.
        replacement: GlyphSet,
    },
    /// GSUB 5 and 6: a contextual substitution, with inline or lookup rules.
    ChainSub {
        /// Glyphs before the input sequence.
        backtrack: Vec<GlyphSet>,
        /// The marked input sequence.
        input: Vec<ContextItem>,
        /// Glyphs after the input sequence.
        lookahead: Vec<GlyphSet>,
        /// An inline replacement written after `by`.
        replacement: Option<Vec<GlyphSet>>,
    },
    /// GSUB 8: a reverse-chaining single substitution.
    ReverseSub {
        /// Glyphs before the input sequence.
        backtrack: Vec<GlyphSet>,
        /// The marked input sequence.
        input: Vec<ContextItem>,
        /// Glyphs after the input sequence.
        lookahead: Vec<GlyphSet>,
        /// An inline replacement written after `by`.
        replacement: Option<Vec<GlyphSet>>,
    },
    /// A GSUB `ignore sub` rule.
    IgnoreSub(Vec<IgnorePattern>),
    /// GPOS 1: a single-glyph adjustment.
    SinglePos {
        /// Adjusted glyphs.
        target: GlyphSet,
        /// The adjustment.
        value: ValueRecord,
    },
    /// GPOS 2: a pair adjustment, optionally enumerated into single pairs.
    PairPos {
        /// Whether the rule was written with `enum`.
        enumerated: bool,
        /// First glyphs.
        first: GlyphSet,
        /// Second glyphs.
        second: GlyphSet,
        /// The value records.
        values: PairValues,
    },
    /// GPOS 3: cursive attachment.
    CursivePos {
        /// Attached glyphs.
        target: GlyphSet,
        /// Entry anchor.
        entry: AnchorPoint,
        /// Exit anchor.
        exit: AnchorPoint,
    },
    /// GPOS 4: mark-to-base attachment.
    MarkToBase {
        /// Base glyphs.
        target: GlyphSet,
        /// Attached mark classes.
        marks: Vec<MarkAttachment>,
    },
    /// GPOS 5: mark-to-ligature attachment, one entry list per component.
    MarkToLigature {
        /// Ligature glyphs.
        target: GlyphSet,
        /// Attached mark classes per ligature component.
        components: Vec<Vec<MarkAttachment>>,
    },
    /// GPOS 6: mark-to-mark attachment.
    MarkToMark {
        /// Base mark glyphs.
        target: GlyphSet,
        /// Attached mark classes.
        marks: Vec<MarkAttachment>,
    },
    /// GPOS 7 and 8: a contextual positioning rule.
    ChainPos {
        /// Glyphs before the input sequence.
        backtrack: Vec<GlyphSet>,
        /// The marked input sequence.
        input: Vec<ContextItem>,
        /// Glyphs after the input sequence.
        lookahead: Vec<GlyphSet>,
    },
    /// A GPOS `ignore pos` rule.
    IgnorePos(Vec<IgnorePattern>),
}

/// A rule together with the lookup type it compiles into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedRule {
    /// The table and lookup type number.
    pub lookup: LookupType,
    /// The rule itself.
    pub rule: Rule,
}

/// A `lookupflag` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupFlag {
    /// The numeric form, including `lookupflag 0;`.
    Value(u16),
    /// The named form.
    Named {
        /// `RightToLeft`.
        right_to_left: bool,
        /// `IgnoreBaseGlyphs`.
        ignore_base_glyphs: bool,
        /// `IgnoreLigatures`.
        ignore_ligatures: bool,
        /// `IgnoreMarks`.
        ignore_marks: bool,
        /// `MarkAttachmentType @class`.
        mark_attachment_type: Option<String>,
        /// `UseMarkFilteringSet @class`.
        use_mark_filtering_set: Option<String>,
    },
}

/// The default-language policy of a `language` statement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LanguagePolicy {
    /// `exclude_dflt`.
    pub exclude_dflt: bool,
    /// `include_dflt`.
    pub include_dflt: bool,
    /// `required`.
    pub required: bool,
}

/// One statement of a feature file, in source order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    /// `languagesystem <script> <language>;`
    LanguageSystem {
        /// Script tag.
        script: String,
        /// Language tag.
        language: String,
    },
    /// `@name = [ ... ];`
    GlyphClassDef {
        /// Class name without the leading at-sign.
        name: String,
        /// Class members.
        members: Vec<ClassItem>,
    },
    /// `markClass <glyphs> <anchor> @class;`
    MarkClassDef {
        /// Mark glyphs.
        glyphs: GlyphSet,
        /// Attachment anchor on the marks.
        anchor: AnchorPoint,
        /// Mark class name without the leading at-sign.
        class: String,
    },
    /// `lookupflag ...;`
    LookupFlag(LookupFlag),
    /// `script <tag>;`
    Script(String),
    /// `language <tag> ...;`
    Language {
        /// Language tag.
        tag: String,
        /// Default-language policy.
        policy: LanguagePolicy,
    },
    /// `subtable;`
    Subtable,
    /// `lookup <name>;`
    LookupCall(String),
    /// `feature <tag>;` inside an `aalt` block.
    FeatureCall(String),
    /// `feature <tag> { ... } <tag>;`
    Feature {
        /// Feature tag.
        tag: String,
        /// Block contents.
        statements: Vec<Statement>,
    },
    /// `lookup <name> [useExtension] { ... } <name>;`
    Lookup {
        /// Lookup name.
        name: String,
        /// Whether `useExtension` was written.
        use_extension: bool,
        /// Block contents.
        statements: Vec<Statement>,
    },
    /// A typed substitution or positioning rule.
    Rule(TypedRule),
    /// A comment, including its leading `#`.
    Comment(String),
    /// Source the IR does not model, kept exactly as written.
    Verbatim(String),
}

/// A parsed feature file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeaIr {
    /// Top-level statements in source order.
    pub statements: Vec<Statement>,
}

impl FeaIr {
    /// Count typed rules, including those nested in features and lookups.
    pub fn typed_rules(&self) -> usize {
        fn count(statements: &[Statement]) -> usize {
            statements
                .iter()
                .map(|statement| match statement {
                    Statement::Rule(_) => 1,
                    Statement::Feature { statements, .. }
                    | Statement::Lookup { statements, .. } => count(statements),
                    _ => 0,
                })
                .sum()
        }
        count(&self.statements)
    }

    /// Count statements kept verbatim because the IR does not model them.
    pub fn verbatim_statements(&self) -> usize {
        fn count(statements: &[Statement]) -> usize {
            statements
                .iter()
                .map(|statement| match statement {
                    Statement::Verbatim(_) => 1,
                    Statement::Feature { statements, .. }
                    | Statement::Lookup { statements, .. } => count(statements),
                    _ => 0,
                })
                .sum()
        }
        count(&self.statements)
    }

    /// Render the IR back to canonical feature syntax.
    pub fn to_fea(&self) -> String {
        write_fea(self)
    }
}
