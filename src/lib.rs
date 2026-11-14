use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::anyhow;
use clap::Parser;
use itertools::{EitherOrBoth, Itertools};
use rand::prelude::SliceRandom;
use serde::Deserialize;

fn parenthetical() -> &'static regex::Regex {
    static PARENTHETICAL: once_cell::sync::OnceCell<regex::Regex> =
        once_cell::sync::OnceCell::new();
    PARENTHETICAL.get_or_init(|| regex::Regex::new(r#"\(.*\)"#).unwrap())
}

#[derive(Debug, Deserialize)]
struct Columns {
    email: String,
    pinch_hitter: String,

    desired_characters: String,
    desired_ships: String,

    banned_characters: String,
    banned_ships: String,

    desired_type: String,
    creation_type: String,
}

fn default_mismatch_threshold() -> f32 {
    1.0
}

#[derive(Debug, Deserialize)]
struct BaseCfg {
    #[serde(default)]
    smushes: Smushes,

    #[serde(default)]
    characters: Characters,

    #[serde(default)]
    families: Families,
}

#[derive(Debug, Deserialize)]
struct Cfg {
    columns: Columns,

    #[serde(default = "default_mismatch_threshold")]
    mismatch_threshold: f32,

    #[serde(flatten)]
    base: BaseCfg,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Misspellings {
    Single(String),
    List(BTreeSet<String>),
}

#[derive(Debug, Deserialize)]
struct Family {
    members: BTreeSet<String>,
    #[serde(default)]
    misspellings: BTreeSet<String>,
    joiner: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Smushes(BTreeMap<String, String>);

#[derive(Debug, Deserialize, Default)]
struct Characters(BTreeMap<String, Misspellings>);

#[derive(Debug, Deserialize, Default)]
struct Families(BTreeMap<String, Family>);

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    data: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    ListShips,
    ListCharacters,
    CheckDuplicates,
    AssignSantas,
    FindPinchHitter {
        #[arg(short, long)]
        giftee: String,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CreationType {
    FanArt,
    Fanfiction,
    Either,
}
impl TryFrom<String> for CreationType {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, anyhow::Error> {
        value.as_str().try_into()
    }
}

impl TryFrom<&str> for CreationType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        use CreationType::*;
        Ok(match value {
            "Fan art" => FanArt,
            "Fanfiction" => Fanfiction,
            "Either" => Either,
            "Fan art, Fanfiction" => Either,
            "Fan art, Either" => Either,
            "Fan art, Fanfiction, Either" => Either,
            other => return Err(anyhow!("No creation type match for: {other}")),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ShipMember {
    Value(String),

    Any,
}

impl<T: Into<String>> From<T> for ShipMember {
    fn from(value: T) -> Self {
        let v = value.into();
        match &v[..] {
            "*" | "any" | "all" | "anyone" => ShipMember::Any,
            _ => ShipMember::Value(v),
        }
    }
}

impl ShipMember {
    fn is_any(&self) -> bool {
        matches!(self, ShipMember::Any)
    }
}

impl std::fmt::Display for ShipMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShipMember::Any => write!(f, "*"),
            ShipMember::Value(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ShipSpec(Vec<ShipMember>);

impl std::fmt::Display for ShipSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for member in &self.0 {
            write!(f, "{member}#")?;
        }
        Ok(())
    }
}

impl ShipSpec {
    fn matches(&self, other: &ShipSpec) -> bool {
        for eob in self.0.iter().zip_longest(other.0.iter()) {
            match eob {
                EitherOrBoth::Both(a, b) => {
                    if a.is_any() || b.is_any() {
                        continue;
                    }
                    if a != b {
                        return false;
                    }
                }
                EitherOrBoth::Left(a) => {
                    if !a.is_any() {
                        return false;
                    }
                }
                EitherOrBoth::Right(b) => {
                    if !b.is_any() {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
struct ShipSet(Vec<ShipSpec>);

impl ShipSet {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn insert(&mut self, spec: ShipSpec) {
        if !self.0.contains(&spec) {
            self.0.push(spec);
        }
        self.0.sort();
    }

    fn difference(&self, other: &ShipSet) -> usize {
        let mut diff = 0;
        'items: for item in &self.0 {
            for removal in &other.0 {
                if removal.matches(item) {
                    continue 'items;
                }
            }
            diff += 1;
        }
        diff
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Participant {
    email: String,
    creation_type: CreationType,
    desired_type: CreationType,
    pinch_hitter: bool,

    desired_characters: BTreeSet<String>,
    desired_ships: ShipSet,
    banned_characters: BTreeSet<String>,
    banned_ships: ShipSet,
}

impl Smushes {
    fn desmush(&self, s: String) -> String {
        self.0.get(&s).cloned().unwrap_or(s)
    }
}

impl Misspellings {
    fn contains(&self, value: &str) -> bool {
        match self {
            Misspellings::Single(s) => s == value,
            Misspellings::List(l) => l.contains(value),
        }
    }
}

impl Characters {
    fn canonicalize(&self, c: String) -> String {
        self.0
            .iter()
            .find_map(|(canonical, misspellings)| {
                if misspellings.contains(&c) {
                    Some(canonical.to_string())
                } else {
                    None
                }
            })
            .unwrap_or(c)
    }
}

impl Family {
    fn matches(&self, name: &str, member: &str, c: &str) -> bool {
        if format!("{name} {member}") == c {
            true
        } else if let Some(ref sep) = self.joiner {
            format!("{name}{sep}{member}") == c
        } else {
            false
        }
    }
}

impl Families {
    fn expand(&self, c: String) -> Vec<String> {
        self.0
            .iter()
            .find_map(|(name, family)| {
                if &c == name || family.misspellings.contains(&c) {
                    Some(family.members.iter().cloned().collect())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| vec![c])
    }

    fn canonicalize(&self, c: String) -> String {
        self.0
            .iter()
            .find_map(|(name, family)| {
                for member in &family.members {
                    if family.matches(name, member, &c) {
                        return Some(member.to_string());
                    }
                    for misspelling in &family.misspellings {
                        if family.matches(misspelling, member, &c) {
                            return Some(member.to_string());
                        }
                    }
                }
                None
            })
            .unwrap_or(c)
    }
}

fn resolve_ships(mut ships: Vec<Vec<String>>) -> Vec<Vec<String>> {
    match ships.pop() {
        None => Vec::new(),
        Some(part) if ships.is_empty() => part.into_iter().map(|p| vec![p]).collect(),
        Some(part) => {
            let rest = resolve_ships(ships);
            let mut result = Vec::new();
            for p in part {
                for s in &rest {
                    let mut next = s.clone();
                    next.push(p.clone());
                    result.push(next);
                }
            }
            result
        }
    }
}

impl BaseCfg {
    fn canonicalize(&self, character: String) -> Vec<String> {
        self.families
            .expand(character)
            .into_iter()
            .map(|character| {
                self.characters
                    .canonicalize(self.families.canonicalize(character))
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn process_characters(&self, characters: String) -> BTreeSet<String> {
        let mut chars = BTreeSet::new();
        for char in parenthetical()
            .replace_all(&characters, "")
            .replace(';', ",")
            .split(',')
        {
            for char in self.canonicalize(char.trim().to_lowercase()) {
                chars.insert(char.trim().to_lowercase());
            }
        }

        chars
    }

    fn process_ships(&self, ships: String) -> ShipSet {
        let mut set = ShipSet::default();
        for chunk in parenthetical()
            .replace_all(&ships, "")
            .replace(';', ",")
            .split(',')
        {
            let chunk = self.smushes.desmush(chunk.trim().to_lowercase());
            let parts: Vec<_> = chunk
                .trim()
                .replace(['&', '\\'], "/")
                .split('/')
                .map(|c| c.trim().to_lowercase())
                .map(|c| self.canonicalize(c))
                .collect();
            for ship in resolve_ships(parts).into_iter() {
                let mut ship = ship.into_iter().map(|item| item.into()).collect::<Vec<_>>();
                ship.sort();
                set.insert(ShipSpec(ship));
            }
        }

        set
    }
}

impl Cfg {
    fn process_participant(
        &self,
        mut row: BTreeMap<String, String>,
    ) -> Result<Participant, anyhow::Error> {
        Ok(Participant {
            email: row
                .remove(&self.columns.email)
                .ok_or_else(|| anyhow!("email column not found"))?,
            pinch_hitter: row
                .remove(&self.columns.pinch_hitter)
                .ok_or_else(|| anyhow!("pinch hitter column not found"))?
                == "Yes",
            creation_type: row
                .remove(&self.columns.creation_type)
                .ok_or_else(|| anyhow!("creation type column not found"))?
                .try_into()?,
            desired_type: row
                .remove(&self.columns.desired_type)
                .ok_or_else(|| anyhow!("creation type column not found"))?
                .try_into()?,

            desired_characters: self.base.process_characters(
                row.remove(&self.columns.desired_characters)
                    .ok_or_else(|| anyhow!("desired characters column not found"))?,
            ),
            banned_characters: self.base.process_characters(
                row.remove(&self.columns.banned_characters)
                    .ok_or_else(|| anyhow!("banned characters column not found"))?,
            ),

            desired_ships: self.base.process_ships(
                row.remove(&self.columns.desired_ships)
                    .ok_or_else(|| anyhow!("desired ships column not found"))?,
            ),
            banned_ships: self.base.process_ships(
                row.remove(&self.columns.banned_ships)
                    .ok_or_else(|| anyhow!("banned ships column not found"))?,
            ),
        })
    }
}

impl Participant {
    fn can_make_for(&self, giftee: &Participant, mismatch_threshold: f32) -> bool {
        if giftee
            .desired_characters
            .difference(&self.banned_characters)
            .count()
            < (giftee.desired_characters.len() as f32 * mismatch_threshold) as usize
        {
            return false;
        }

        if giftee.desired_ships.difference(&self.banned_ships)
            < (giftee.desired_ships.len() as f32 * mismatch_threshold) as usize
        {
            return false;
        }

        match (self.creation_type, giftee.desired_type) {
            (CreationType::FanArt, CreationType::Fanfiction)
            | (CreationType::Fanfiction, CreationType::FanArt) => return false,

            (CreationType::Either, _)
            | (_, CreationType::Either)
            | (CreationType::FanArt, CreationType::FanArt)
            | (CreationType::Fanfiction, CreationType::Fanfiction) => (),
        }

        true
    }
}

pub fn run() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let cfg: Cfg = toml::from_str(&std::fs::read_to_string("cfg.toml")?)?;

    let data: Vec<Participant> = csv::Reader::from_path(&args.data)?
        .into_deserialize::<BTreeMap<String, String>>()
        .map(|r| r.map_err(|e| anyhow!("{e:?}")))
        .map(|r| r.and_then(|r| cfg.process_participant(r)))
        .collect::<Result<_, anyhow::Error>>()?;

    match args.command {
        Commands::ListCharacters => {
            let mut characters = BTreeMap::<_, u32>::new();
            for row in data {
                for char in row.desired_characters.into_iter() {
                    *characters.entry(char).or_default() += 1;
                }
                for char in row.banned_characters.into_iter() {
                    *characters.entry(char).or_default() += 1;
                }
            }
            let mut characters: Vec<_> = characters.into_iter().collect();
            characters.sort_by(|a, b| a.1.cmp(&b.1));
            for (k, v) in characters {
                println!("{k}: {v}");
            }
        }
        Commands::ListShips => {
            let mut ships = BTreeMap::<_, u32>::new();
            for row in data {
                for ship in row.desired_ships.0.into_iter() {
                    *ships.entry(ship).or_default() += 1;
                }
                for ship in row.banned_ships.0.into_iter() {
                    *ships.entry(ship).or_default() += 1;
                }
            }
            let mut ships: Vec<_> = ships.into_iter().collect();
            ships.sort_by(|a, b| a.1.cmp(&b.1));
            for (k, v) in ships {
                println!("{k}: {v}");
            }
        }
        Commands::CheckDuplicates => {
            let mut emails = BTreeMap::<_, u32>::new();
            for row in data {
                *emails.entry(row.email).or_default() += 1;
            }
            let mut emails: Vec<_> = emails.into_iter().collect();
            emails.sort_by(|a, b| a.1.cmp(&b.1));
            let mut found_dupe = false;
            for (k, v) in emails {
                if v > 1 {
                    if !found_dupe {
                        println!("Duplicate Entries");
                    }
                    found_dupe = true;
                    println!("{k}: {v}");
                }
            }
            if !found_dupe {
                println!("✨ No Duplicates! ✨");
            }
        }
        Commands::FindPinchHitter { giftee } => {
            let mut rng = rand::thread_rng();
            let mut candidates = Vec::new();
            for (a, b) in itertools::iproduct!(&data, &data) {
                if a == b {
                    continue;
                }
                if b.email != giftee {
                    continue;
                }
                if a.can_make_for(b, cfg.mismatch_threshold) && a.pinch_hitter {
                    candidates.push(a.email.clone());
                }
            }
            candidates.shuffle(&mut rng);
            for candidate in candidates {
                println!("{candidate}");
            }
        }
        Commands::AssignSantas => {
            let mut rng = rand::thread_rng();
            let mut matches = BTreeMap::new();
            for (a, b) in itertools::iproduct!(&data, &data) {
                if a == b {
                    continue;
                }
                if a.can_make_for(b, cfg.mismatch_threshold) {
                    matches
                        .entry(a.email.clone())
                        .or_insert_with(Vec::new)
                        .push(b.email.clone());
                }
            }
            for (_, l) in matches.iter_mut() {
                l.shuffle(&mut rng);
            }
            let problem_child = matches
                .iter()
                .min_by(|x, y| x.1.len().cmp(&y.1.len()))
                .ok_or(anyhow::format_err!("Could not find problem child"))?;

            fn match_from(
                graph: &BTreeMap<String, Vec<String>>,
                current_node: String,
                used: &mut BTreeSet<String>,
            ) -> Option<VecDeque<String>> {
                if used.len() == graph.len() {
                    return Some(VecDeque::from([current_node]));
                }
                for next_hop in &graph[&current_node] {
                    if used.contains(next_hop) {
                        continue;
                    }
                    used.insert(next_hop.to_string());
                    if let Some(mut solution) = match_from(graph, next_hop.clone(), used) {
                        solution.push_front(current_node);
                        return Some(solution);
                    }
                    used.remove(next_hop);
                }

                None
            }

            if let Some(solution) = match_from(
                &matches,
                problem_child.0.to_string(),
                &mut BTreeSet::from([problem_child.0.to_string()]),
            ) {
                println!();
                println!();
                println!("Solution:");
                for creator in solution {
                    println!("{creator}");
                }
            } else {
                println!("No solution found!");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::*;
    use pretty_assertions::assert_eq;

    fn cfg() -> BaseCfg {
        toml::from_str(
            r#"
[families."monkey d."]
members = ["dragon", "garp", "luffy"]
misspellings = ["monkey d"]

[characters]
ace = ["portgas ace", "portgas d. ace"]
"boa hancock" = "boa"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_families_expand() {
        let families = cfg().families;
        assert_eq!(
            families.expand("monkey d".to_string()),
            &["dragon", "garp", "luffy"],
        );
    }

    #[test]
    fn test_families_canonicalize() {
        let cfg = cfg().families;

        assert_eq!(cfg.canonicalize("monkey d luffy".to_string()), "luffy");
        assert_eq!(cfg.canonicalize("monkey d. garp".to_string()), "garp");
    }

    #[test]
    fn test_basecfg_canonicalize() {
        let cfg = cfg();

        assert_eq!(
            cfg.canonicalize("monkey d luffy".to_string()),
            vec!["luffy"]
        );
        assert_eq!(cfg.canonicalize("monkey d. garp".to_string()), vec!["garp"]);
        assert_eq!(cfg.canonicalize("boa".to_string()), vec!["boa hancock"]);
    }

    #[test]
    fn test_basecfg_process_characters() {
        let cfg = cfg();

        assert_eq!(
            cfg.process_characters("Monkey d luffy, garp, monkey d. dragon".into()),
            ["luffy".into(), "garp".into(), "dragon".into()].into()
        );

        assert_eq!(
            cfg.process_characters("portgas ace,portgas d. ace".into()),
            ["ace".into()].into()
        );
    }

    #[test]
    fn test_basecfg_process_ships() {
        let cfg = cfg();

        assert_eq!(
            cfg.process_ships("boa/luffy,monkey d/ace".into()),
            ShipSet(vec![
                ShipSpec(vec!["ace".into(), "dragon".into(),]),
                ShipSpec(vec!["ace".into(), "garp".into(),]),
                ShipSpec(vec!["ace".into(), "luffy".into()]),
                ShipSpec(vec!["boa hancock".into(), "luffy".into(),]),
            ])
        );
    }
}
