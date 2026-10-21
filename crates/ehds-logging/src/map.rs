// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The deployment's category map: which categories the data under a template
//! or an archetype belong to (Regulation (EU) 2025/327 Annex II 3.2(c)).
//!
//! No adopted act and no openEHR specification ties a template or an
//! archetype to an Art 14(1) category, so the deployment declares it: each
//! template id and each archetype id maps to a set of categories, or to
//! `none`, the deployment's statement that the data under it are of no
//! category. The crate ships no map. A template key wins over an archetype
//! key for the same object, because a template is the clinical context the
//! archetype is used in (no specification governs the map: our own design).
//!
//! A key is compared exactly by default. Declared with [`Matching::OpenEhr`],
//! the map compares template and archetype ids as openEHR BASE asks of every
//! composite identifier: case-insensitively, while the evidence keeps each id
//! as the access named it (BASE Release 1.3.0, Base Types, Identification
//! Package, "Composite Identifiers and Case"). Archetype keys are held to the
//! `ARCHETYPE_ID` grammar of the same package either way.
//!
//! The map's [`CategoryMap::canonical`] text is the same for the same
//! declarations in any order, so a digest of it names the map a record was
//! classified under.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};

use crate::category::{Category, CodeError, NationalCategory, Reference};

/// The word a declaration uses for data of no category.
pub const NONE: &str = "none";

/// One declaration as a configuration writes it: the word `none`, or a list
/// of categories.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Declared {
    /// A word, of which only [`NONE`] is admitted.
    Word(String),
    /// A list of categories, at least one: a priority category by its bare
    /// code or as `<system>|<code>`, a national one as `<system>|<code>`.
    Codes(Vec<String>),
}

/// What a key maps to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mapping {
    /// The data under the key belong to these categories.
    Categories(BTreeSet<Category>),
    /// The data under the key belong to no category.
    NoCategory,
}

/// How a map compares an id with its keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Matching {
    /// Exactly: an id that differs from a key by case is unmapped.
    #[default]
    Exact,
    /// As openEHR BASE compares composite identifiers: two ids identical
    /// apart from case identify the same thing. BASE limits their textual
    /// parts to the basic latin letters, so the comparison folds ASCII case.
    OpenEhr,
}

impl Matching {
    /// `id` as this matching compares it.
    fn key(self, id: &str) -> String {
        match self {
            Self::Exact => id.to_owned(),
            Self::OpenEhr => id.to_ascii_lowercase(),
        }
    }
}

/// The two tables a map keys on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    /// Template ids.
    Templates,
    /// Archetype ids.
    Archetypes,
}

impl Table {
    /// The table's name, as a configuration and an error name it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Templates => "templates",
            Self::Archetypes => "archetypes",
        }
    }
}

/// Why a map was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MapError {
    /// A key is empty or holds a control character.
    #[error("a key of the {} table is empty or holds a control character", .0.name())]
    Key(Table),
    /// A key maps to a word other than `none`.
    #[error("{} key {key:?} maps to a word other than \"none\"", .table.name())]
    Word {
        /// The table.
        table: Table,
        /// The key.
        key: String,
    },
    /// A key maps to an empty list.
    #[error("{} key {key:?} maps to no category; write \"none\" for data of no category", .table.name())]
    Empty {
        /// The table.
        table: Table,
        /// The key.
        key: String,
    },
    /// A key maps to a code that is neither a priority category nor a
    /// declared national one.
    #[error("{} key {key:?} maps to {code:?}, which is neither a priority category nor a declared national one", .table.name())]
    Unknown {
        /// The table.
        table: Table,
        /// The key.
        key: String,
        /// The code.
        code: String,
    },
    /// A declared national category is refused.
    #[error("the national category {code:?} is refused")]
    National {
        /// The category as declared.
        code: String,
        /// Why.
        #[source]
        source: CodeError,
    },
    /// An archetype key is no `ARCHETYPE_ID`.
    #[error(
        "archetypes key {key:?} is no openEHR archetype id ([namespace::]rm_originator-rm_name-rm_entity.concept.vN)"
    )]
    ArchetypeId {
        /// The key.
        key: String,
    },
    /// Two keys of a table name the same id under the map's matching.
    #[error("{} keys {first:?} and {second:?} name the same id", .table.name())]
    DuplicateKey {
        /// The table.
        table: Table,
        /// The first key.
        first: String,
        /// The key that names the same id.
        second: String,
    },
}

/// Every defect of a refused map declaration, in table and key order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapErrors(Vec<MapError>);

impl MapErrors {
    /// Each defect.
    #[must_use]
    pub fn errors(&self) -> &[MapError] {
        &self.0
    }

    /// The first defect.
    #[must_use]
    pub fn first(&self) -> Option<&MapError> {
        self.0.first()
    }

    /// The first defect, taken.
    #[expect(
        clippy::expect_used,
        reason = "declare_with returns MapErrors only after pushing a defect, so it is never empty"
    )]
    fn into_first(self) -> MapError {
        self.0
            .into_iter()
            .next()
            .expect("a refused declaration should hold a defect")
    }
}

impl fmt::Display for MapErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the category map has {} defects", self.0.len())?;
        for error in &self.0 {
            write!(f, "; {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for MapErrors {}

impl IntoIterator for MapErrors {
    type Item = MapError;
    type IntoIter = std::vec::IntoIter<MapError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// The deployment's category map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CategoryMap {
    national: BTreeSet<NationalCategory>,
    matching: Matching,
    templates: BTreeMap<String, (String, Mapping)>,
    archetypes: BTreeMap<String, (String, Mapping)>,
    digest: Option<String>,
    evidence_limit: Option<usize>,
}

impl CategoryMap {
    /// The map `templates` and `archetypes` declare, their categories read
    /// against the six priority categories and the `national` ones, each
    /// declared `<system>|<code>`, its keys compared exactly.
    ///
    /// # Errors
    ///
    /// The first [`MapError`] [`CategoryMap::declare_with`] finds.
    pub fn declare(
        national: &[String],
        templates: &BTreeMap<String, Declared>,
        archetypes: &BTreeMap<String, Declared>,
    ) -> Result<Self, MapError> {
        Self::declare_with(Matching::Exact, national, templates, archetypes)
            .map_err(MapErrors::into_first)
    }

    /// The map `templates` and `archetypes` declare, compared by `matching`,
    /// their categories read against the six priority categories and the
    /// `national` ones, each declared `<system>|<code>`.
    ///
    /// # Errors
    ///
    /// [`MapErrors`] with every defect: a refused national code, an empty or
    /// malformed key, an archetype key that is no `ARCHETYPE_ID`, two keys that
    /// name the same id, a word other than `none`, an empty list, and a code
    /// that names no category.
    pub fn declare_with(
        matching: Matching,
        national: &[String],
        templates: &BTreeMap<String, Declared>,
        archetypes: &BTreeMap<String, Declared>,
    ) -> Result<Self, MapErrors> {
        let mut errors = Vec::new();
        let mut declared = BTreeSet::new();
        for code in national {
            match code.parse::<NationalCategory>() {
                Ok(category) => {
                    declared.insert(category);
                }
                Err(source) => errors.push(MapError::National {
                    code: code.clone(),
                    source,
                }),
            }
        }
        let mut read = |table: Table, keys: &BTreeMap<String, Declared>| {
            let mut read: BTreeMap<String, (String, Mapping)> = BTreeMap::new();
            for (key, value) in keys {
                if table == Table::Archetypes && !is_archetype_id(key) && is_key(key) {
                    errors.push(MapError::ArchetypeId { key: key.clone() });
                    continue;
                }
                match mapping(table, key, value, &declared) {
                    Ok(mapping) => {
                        let folded = matching.key(key);
                        if let Some((first, _)) = read.get(&folded) {
                            errors.push(MapError::DuplicateKey {
                                table,
                                first: first.clone(),
                                second: key.clone(),
                            });
                        } else {
                            read.insert(folded, (key.clone(), mapping));
                        }
                    }
                    Err(error) => errors.push(error),
                }
            }
            read
        };
        let templates = read(Table::Templates, templates);
        let archetypes = read(Table::Archetypes, archetypes);
        if !errors.is_empty() {
            return Err(MapErrors(errors));
        }
        Ok(Self {
            national: declared,
            matching,
            templates,
            archetypes,
            digest: None,
            evidence_limit: None,
        })
    }

    /// This map, keeping at most `limit` ids in each evidence set of a
    /// classification, with each set's true count beside it.
    #[must_use]
    pub fn with_evidence_limit(mut self, limit: usize) -> Self {
        self.evidence_limit = Some(limit);
        self
    }

    /// The most ids a classification keeps in each evidence set, when the
    /// map bounds them.
    #[must_use]
    pub fn evidence_limit(&self) -> Option<usize> {
        self.evidence_limit
    }

    /// How the map compares an id with its keys.
    #[must_use]
    pub fn matching(&self) -> Matching {
        self.matching
    }

    /// Every template key, as declared, with what it maps to, in key order.
    pub fn templates(&self) -> impl Iterator<Item = (&str, &Mapping)> {
        self.templates
            .values()
            .map(|(key, mapping)| (key.as_str(), mapping))
    }

    /// Every archetype key, as declared, with what it maps to, in key order.
    pub fn archetypes(&self) -> impl Iterator<Item = (&str, &Mapping)> {
        self.archetypes
            .values()
            .map(|(key, mapping)| (key.as_str(), mapping))
    }

    /// Every national category the map declares, in order.
    pub fn national(&self) -> impl Iterator<Item = &NationalCategory> {
        self.national.iter()
    }

    /// This map, named by `digest`, such as a hash of
    /// [`CategoryMap::canonical`].
    #[must_use]
    pub fn with_digest(mut self, digest: String) -> Self {
        self.digest = Some(digest);
        self
    }

    /// The digest the map is named by, when one was given.
    #[must_use]
    pub fn digest(&self) -> Option<&str> {
        self.digest.as_deref()
    }

    /// What `template_id` maps to, when the map holds it under its
    /// matching.
    #[must_use]
    pub fn template(&self, template_id: &str) -> Option<&Mapping> {
        self.templates
            .get(&self.matching.key(template_id))
            .map(|(_, mapping)| mapping)
    }

    /// What `archetype_id` maps to, when the map holds it under its
    /// matching.
    #[must_use]
    pub fn archetype(&self, archetype_id: &str) -> Option<&Mapping> {
        self.archetypes
            .get(&self.matching.key(archetype_id))
            .map(|(_, mapping)| mapping)
    }

    /// The category `reference` names: a priority category by its bare code
    /// or as `<system>|<code>`, or a national one the map declares, as
    /// `<system>|<code>`. Codes are compared exactly.
    #[must_use]
    pub fn category(&self, reference: &str) -> Option<Category> {
        named(reference, &self.national)
    }

    /// Every category the map can name: the six priority categories of Art
    /// 14(1) in their order, then each national category it declares.
    pub fn categories(&self) -> impl Iterator<Item = Category> + '_ {
        Category::PRIORITY
            .into_iter()
            .chain(self.national.iter().cloned().map(Category::National))
    }

    /// Whether the map holds no key.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty() && self.archetypes.is_empty()
    }

    /// The map as text that is the same for the same declarations: a line
    /// `matching\topenehr` for a map compared as openEHR BASE asks, one line
    /// per national category, then per template and per archetype key, each
    /// field separated by a tab, the categories by `,`, each category as
    /// `<system>|<code>`. A key is written as the map compares it: as
    /// declared in an exact map, and in its comparison form in an openEHR one,
    /// so two declarations that differ only in the case of a key have one
    /// text.
    #[must_use]
    pub fn canonical(&self) -> String {
        let mut text = String::new();
        if self.matching == Matching::OpenEhr {
            text.push_str("matching\topenehr\n");
        }
        for national in &self.national {
            let _written = writeln!(text, "national\t{}|{}", national.system(), national.code());
        }
        for (table, keys) in [
            (Table::Templates, &self.templates),
            (Table::Archetypes, &self.archetypes),
        ] {
            // NOTE: openEHR BASE 1.3.0 Base Types, Composite Identifiers and Case: under
            // Matching::OpenEhr a key is written in its comparison form, so one map has one text.
            for (key, (_, mapping)) in keys {
                let value = match mapping {
                    Mapping::NoCategory => NONE.to_owned(),
                    Mapping::Categories(categories) => categories
                        .iter()
                        .map(Category::token)
                        .collect::<Vec<_>>()
                        .join(","),
                };
                let _written = writeln!(text, "{}\t{key}\t{value}", table.name());
            }
        }
        text
    }
}

/// The mapping `declared` states for `key` of `table`.
fn mapping(
    table: Table,
    key: &str,
    declared: &Declared,
    national: &BTreeSet<NationalCategory>,
) -> Result<Mapping, MapError> {
    if !is_key(key) {
        return Err(MapError::Key(table));
    }
    let codes = match declared {
        Declared::Word(word) if word == NONE => return Ok(Mapping::NoCategory),
        Declared::Word(_) => {
            return Err(MapError::Word {
                table,
                key: key.to_owned(),
            });
        }
        Declared::Codes(codes) if codes.is_empty() => {
            return Err(MapError::Empty {
                table,
                key: key.to_owned(),
            });
        }
        Declared::Codes(codes) => codes,
    };
    codes
        .iter()
        .map(|code| {
            named(code, national).ok_or_else(|| MapError::Unknown {
                table,
                key: key.to_owned(),
                code: code.clone(),
            })
        })
        .collect::<Result<BTreeSet<_>, _>>()
        .map(Mapping::Categories)
}

/// Whether `key` is not empty and holds no control character.
fn is_key(key: &str) -> bool {
    !key.is_empty() && !key.chars().any(char::is_control)
}

/// Whether `id` follows the `ARCHETYPE_ID` grammar: the namespaced ADL 2 form
/// `namespace::rm_originator-rm_name-rm_entity.concept.vM.m.p-rc.N` or the
/// legacy form `rm_originator-rm_name-rm_entity.concept.vM`.
// NOTE: openEHR BASE 1.3.0, Base Types, Identification Package, Syntaxes; a namespaceless id
// with a full version is admitted as FerroEHR's ArchetypeId admits it (AM 2.4.0 ARCHETYPE_HRID).
fn is_archetype_id(id: &str) -> bool {
    let alphanumeric = |part: &str| {
        let mut characters = part.chars();
        characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
    };
    let extended = |part: &str| {
        let mut characters = part.chars();
        characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_' || rest == '-')
    };
    let (namespace, rest) = match id.split_once("::") {
        Some((namespace, rest)) => (Some(namespace), rest),
        None => (None, id),
    };
    if namespace.is_some_and(|namespace| !namespace.split('.').all(extended)) {
        return false;
    }
    let mut sections = rest.splitn(3, '.');
    let (Some(entity), Some(concept), Some(version)) =
        (sections.next(), sections.next(), sections.next())
    else {
        return false;
    };
    let mut entity_parts = entity.split('-');
    let entity_ok = matches!(
        (entity_parts.next(), entity_parts.next(), entity_parts.next(), entity_parts.next()),
        (Some(originator), Some(name), Some(class), None)
            if alphanumeric(originator) && alphanumeric(name) && alphanumeric(class)
    );
    let concept_ok = concept.split('-').all(alphanumeric);
    let version_ok = version.strip_prefix('v').is_some_and(is_version_id);
    entity_ok && concept_ok && version_ok
}

/// Whether `version` is a BASE `version_id`: `major [. minor [. patch
/// [modifier]]]`, each number `0` or without a leading zero, and the modifier
/// `-rc`, `-alpha` or `-beta` with an optional `.build`.
fn is_version_id(version: &str) -> bool {
    let number = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|digit| digit.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    };
    let (numeric, modifier) = match version.split_once('-') {
        Some((numeric, modifier)) => (numeric, Some(modifier)),
        None => (version, None),
    };
    let mut count = 0_usize;
    for part in numeric.split('.') {
        count = count.saturating_add(1);
        if !number(part) {
            return false;
        }
    }
    let modifier_ok = modifier.is_none_or(|modifier| {
        let (kind, build) = match modifier.split_once('.') {
            Some((kind, build)) => (kind, Some(build)),
            None => (modifier, None),
        };
        count == 3
            && matches!(kind, "rc" | "alpha" | "beta")
            && build.is_none_or(|build| {
                !build.is_empty() && build.bytes().all(|digit| digit.is_ascii_digit())
            })
    });
    (1..=3).contains(&count) && modifier_ok
}

/// The category `reference` names among the priority categories and the
/// `national` ones.
fn named(reference: &str, national: &BTreeSet<NationalCategory>) -> Option<Category> {
    let reference = Reference::read(reference);
    reference.priority().or_else(|| {
        national
            .iter()
            .find(|declared| reference.names(declared))
            .map(|declared| Category::National(declared.clone()))
    })
}
