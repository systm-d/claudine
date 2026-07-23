//! Statistiques d'usage : agrégation des tokens consommés par les sessions
//! Claude Code, estimation de coût et grille d'activité (façon GitHub).
//!
//! Claude Code inscrit dans chaque `.jsonl` de session, pour chaque message
//! assistant, un bloc `message.usage` (`input_tokens`, `output_tokens`,
//! `cache_creation_input_tokens`, `cache_read_input_tokens`) et le `message.model`
//! utilisé. Ce module lit ces compteurs — ignorés par `scan.rs`, qui ne récolte
//! que les métadonnées — et les agrège par modèle et par jour.
//!
//! L'estimation de coût s'appuie sur une table de tarifs par famille de modèle
//! (USD par million de tokens). Les modèles inconnus de la table sont comptés en
//! tokens mais exclus du coût (et signalés), pour ne jamais présenter un montant
//! faussement précis.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

/// Compteurs de tokens d'un message, d'une session ou d'un agrégat.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    /// Tokens écrits dans le cache (facturés ~1,25× le prix d'entrée).
    pub cache_creation: u64,
    /// Tokens lus depuis le cache (facturés ~0,1× le prix d'entrée).
    pub cache_read: u64,
}

impl Tokens {
    /// Somme de tous les compteurs (entrée + sortie + caches).
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_creation + self.cache_read
    }

    fn add(&mut self, other: Tokens) {
        self.input += other.input;
        self.output += other.output;
        self.cache_creation += other.cache_creation;
        self.cache_read += other.cache_read;
    }
}

/// Tarif d'un modèle, en USD par million de tokens.
#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    pub input: f64,
    pub output: f64,
}

impl ModelPrice {
    /// Coût estimé (USD) de `t` à ce tarif. Les écritures de cache sont
    /// facturées 1,25× l'entrée (TTL 5 min, le défaut) et les lectures 0,1×.
    pub fn cost(&self, t: Tokens) -> f64 {
        (t.input as f64 * self.input
            + t.output as f64 * self.output
            + t.cache_creation as f64 * self.input * 1.25
            + t.cache_read as f64 * self.input * 0.1)
            / 1_000_000.0
    }
}

/// Tarif de `model`, choisi par famille (Opus/Sonnet/Haiku/Fable) et version.
/// `None` pour un modèle non reconnu (ex. `<synthetic>`) : compté en tokens
/// mais exclu du coût. Tarifs Anthropic first-party (USD / million de tokens).
pub fn price_for(model: &str) -> Option<ModelPrice> {
    let m = model.to_ascii_lowercase();
    let p = |input, output| Some(ModelPrice { input, output });
    if m.contains("fable") || m.contains("mythos") {
        return p(10.0, 50.0);
    }
    if m.contains("opus") {
        // Opus 4.5 à 4.8 : 5/25. Opus 4.1/4.0 et Opus 3 : 15/75 (tarif historique).
        if m.contains("opus-4-8")
            || m.contains("opus-4-7")
            || m.contains("opus-4-6")
            || m.contains("opus-4-5")
        {
            return p(5.0, 25.0);
        }
        return p(15.0, 75.0);
    }
    if m.contains("sonnet") {
        return p(3.0, 15.0);
    }
    if m.contains("haiku") {
        if m.contains("haiku-4") {
            return p(1.0, 5.0);
        }
        if m.contains("3-5-haiku") || m.contains("haiku-3-5") {
            return p(0.80, 4.0);
        }
        if m.contains("3-haiku") || m.contains("haiku-3") {
            return p(0.25, 1.25);
        }
        return p(1.0, 5.0);
    }
    None
}

/// Agrégat d'usage, cumulable : par modèle, par jour, plus le total de messages
/// assistant. Sert aussi bien à une session isolée qu'à l'ensemble des homes.
#[derive(Debug, Clone, Default)]
pub struct Usage {
    per_model: BTreeMap<String, Tokens>,
    msgs_per_model: BTreeMap<String, usize>,
    /// Tokens par jour (`YYYY-MM-DD`), pour la grille d'activité.
    daily: BTreeMap<String, Tokens>,
    assistant_messages: usize,
}

impl Usage {
    /// Enregistre les tokens d'un message assistant du `model` donné, daté par
    /// `ts` (RFC 3339 ; seule la partie `YYYY-MM-DD` est retenue).
    pub fn record(&mut self, model: &str, ts: Option<&str>, t: Tokens) {
        self.assistant_messages += 1;
        self.per_model.entry(model.to_string()).or_default().add(t);
        *self.msgs_per_model.entry(model.to_string()).or_default() += 1;
        if let Some(day) = ts.and_then(day_of) {
            self.daily.entry(day).or_default().add(t);
        }
    }

    /// Fusionne un autre agrégat dans celui-ci.
    pub fn merge(&mut self, other: &Usage) {
        self.assistant_messages += other.assistant_messages;
        for (m, t) in &other.per_model {
            self.per_model.entry(m.clone()).or_default().add(*t);
        }
        for (m, n) in &other.msgs_per_model {
            *self.msgs_per_model.entry(m.clone()).or_default() += n;
        }
        for (d, t) in &other.daily {
            self.daily.entry(d.clone()).or_default().add(*t);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.assistant_messages == 0
    }

    pub fn assistant_messages(&self) -> usize {
        self.assistant_messages
    }

    /// Total tous modèles confondus.
    pub fn totals(&self) -> Tokens {
        let mut t = Tokens::default();
        for v in self.per_model.values() {
            t.add(*v);
        }
        t
    }

    /// Coût total estimé (USD), sur les seuls modèles tarifés.
    pub fn total_cost(&self) -> f64 {
        self.per_model
            .iter()
            .filter_map(|(m, t)| price_for(m).map(|p| p.cost(*t)))
            .sum()
    }

    /// Vrai si au moins un modèle rencontré n'a pas de tarif connu (coût partiel).
    pub fn has_unpriced(&self) -> bool {
        self.per_model.keys().any(|m| price_for(m).is_none())
    }

    /// Lignes par modèle, triées par total de tokens décroissant.
    pub fn model_rows(&self) -> Vec<ModelRow> {
        let mut rows: Vec<ModelRow> = self
            .per_model
            .iter()
            .map(|(model, t)| ModelRow {
                model: model.clone(),
                tokens: *t,
                messages: self.msgs_per_model.get(model).copied().unwrap_or(0),
                cost: price_for(model).map(|p| p.cost(*t)),
            })
            .collect();
        rows.sort_by(|a, b| {
            b.tokens
                .total()
                .cmp(&a.tokens.total())
                .then_with(|| a.model.cmp(&b.model))
        });
        rows
    }

    /// Jour d'activité le plus ancien / le plus récent (`YYYY-MM-DD`).
    pub fn date_span(&self) -> Option<(String, String)> {
        let first = self.daily.keys().next()?.clone();
        let last = self.daily.keys().next_back()?.clone();
        Some((first, last))
    }

    /// Total de tokens un jour donné (0 si aucune activité).
    pub fn day_total(&self, day: &str) -> u64 {
        self.daily.get(day).map(Tokens::total).unwrap_or(0)
    }

    /// Jours d'activité (`YYYY-MM-DD`, total de tokens), triés chronologiquement.
    pub fn daily_rows(&self) -> Vec<(String, u64)> {
        self.daily
            .iter()
            .map(|(d, t)| (d.clone(), t.total()))
            .collect()
    }

    /// Construit une grille d'activité de `weeks` semaines se terminant à
    /// `anchor` (inclus), calée sur la semaine (dimanche en tête, façon GitHub).
    /// Chaque cellule porte sa date, son total de tokens et un niveau 0–4
    /// d'intensité (relatif au jour le plus chargé de la grille).
    pub fn heatmap(&self, anchor: Date, weeks: usize) -> Heatmap {
        // Fin de grille : le samedi de la semaine d'`anchor` (colonne pleine).
        let end = anchor.days() + (6 - anchor.weekday() as i64);
        let start = end - (weeks as i64 * 7 - 1);

        let mut cells: Vec<Vec<HeatCell>> = Vec::with_capacity(weeks);
        let mut max = 0u64;
        let mut d = start;
        for _ in 0..weeks {
            let mut col = Vec::with_capacity(7);
            for _ in 0..7 {
                let date = Date::from_days(d);
                let key = date.ymd_string();
                let tokens = self.day_total(&key);
                max = max.max(tokens);
                col.push(HeatCell {
                    date: key,
                    days: d,
                    tokens,
                    level: 0,
                    in_range: d <= anchor.days(),
                });
                d += 1;
            }
            cells.push(col);
        }
        // Niveaux relatifs au maximum (quatre teintes au-dessus de zéro).
        for col in &mut cells {
            for c in col {
                c.level = level_for(c.tokens, max);
            }
        }
        Heatmap {
            weeks: cells,
            max,
            start: Date::from_days(start),
            end: Date::from_days(end),
        }
    }
}

/// Une ligne du tableau par modèle.
#[derive(Debug, Clone)]
pub struct ModelRow {
    pub model: String,
    pub tokens: Tokens,
    pub messages: usize,
    /// Coût estimé (USD), ou `None` si le modèle n'a pas de tarif connu.
    pub cost: Option<f64>,
}

/// Grille d'activité prête à afficher : colonnes = semaines, lignes = jours.
#[derive(Debug, Clone)]
pub struct Heatmap {
    /// `weeks[semaine][jour_de_semaine]`, dimanche = index 0.
    pub weeks: Vec<Vec<HeatCell>>,
    pub max: u64,
    pub start: Date,
    pub end: Date,
}

/// Une case de la grille d'activité.
#[derive(Debug, Clone)]
pub struct HeatCell {
    pub date: String,
    days: i64,
    pub tokens: u64,
    /// 0 (aucune activité) à 4 (jour le plus chargé).
    pub level: u8,
    /// Faux pour les jours postérieurs à l'ancre (cases futures, non peintes).
    pub in_range: bool,
}

impl HeatCell {
    pub fn days(&self) -> i64 {
        self.days
    }
}

fn level_for(tokens: u64, max: u64) -> u8 {
    if tokens == 0 || max == 0 {
        return 0;
    }
    // 1..=4, proportionnel au maximum du panneau.
    let lvl = (4 * tokens).div_ceil(max);
    lvl.clamp(1, 4) as u8
}

/// Extrait le jour (`YYYY-MM-DD`) d'un horodatage RFC 3339, si plausible.
fn day_of(ts: &str) -> Option<String> {
    let head: String = ts.chars().take(10).collect();
    let ok = head.len() == 10
        && head.as_bytes()[4] == b'-'
        && head.as_bytes()[7] == b'-'
        && head
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
    ok.then_some(head)
}

/// Lit l'usage d'une session `.jsonl`. Erreurs d'I/O → usage vide (la session
/// est simplement ignorée dans l'agrégat).
pub fn read_session_usage(path: &Path) -> Usage {
    match fs::read_to_string(path) {
        Ok(content) => usage_from_jsonl(&content),
        Err(_) => Usage::default(),
    }
}

/// Extrait l'usage du contenu brut d'un `.jsonl`. Chaque ligne assistant
/// portant un `message.usage` est comptée. Les identifiants `message.id`
/// répétés (reprises de session, sidechains) sont dédoublonnés pour ne pas
/// gonfler les totaux.
pub fn usage_from_jsonl(content: &str) -> Usage {
    let mut usage = Usage::default();
    let mut seen: HashSet<String> = HashSet::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(msg) = v.get("message") else {
            continue;
        };
        let Some(u) = msg.get("usage") else {
            continue;
        };
        // Dédoublonnage par identifiant de message (quand présent).
        if let Some(id) = msg.get("id").and_then(Value::as_str) {
            if !seen.insert(id.to_string()) {
                continue;
            }
        }
        let field = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
        let tokens = Tokens {
            input: field("input_tokens"),
            output: field("output_tokens"),
            cache_creation: field("cache_creation_input_tokens"),
            cache_read: field("cache_read_input_tokens"),
        };
        if tokens.total() == 0 {
            continue;
        }
        let model = msg
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("(inconnu)");
        let ts = v.get("timestamp").and_then(Value::as_str);
        usage.record(model, ts, tokens);
    }
    usage
}

// --- Dates civiles (grégorien proleptique), sans dépendance externe ---

/// Un jour civil, représenté par son décalage en jours depuis 1970-01-01.
/// Algorithmes de conversion « days_from_civil » (Howard Hinnant), valables
/// sur toute la plage utile ; suffisent au calcul de la grille d'activité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date(i64);

impl Date {
    /// Jour civil depuis (année, mois 1–12, jour 1–31).
    pub fn from_ymd(y: i64, m: u32, d: u32) -> Date {
        let yy = if m <= 2 { y - 1 } else { y };
        let era = (if yy >= 0 { yy } else { yy - 399 }) / 400;
        let yoe = yy - era * 400; // [0, 399]
        let mp = if m > 2 { m - 3 } else { m + 9 }; // mars = 0
        let doy = (153 * mp as i64 + 2) / 5 + d as i64 - 1; // [0, 365]
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
        Date(era * 146097 + doe - 719468)
    }

    /// Parse un jour `YYYY-MM-DD` (le reste de l'horodatage est ignoré).
    pub fn parse(day: &str) -> Option<Date> {
        let d = day_of(day)?;
        let y: i64 = d[0..4].parse().ok()?;
        let m: u32 = d[5..7].parse().ok()?;
        let dd: u32 = d[8..10].parse().ok()?;
        Some(Date::from_ymd(y, m, dd))
    }

    pub fn from_days(z: i64) -> Date {
        Date(z)
    }

    pub fn days(&self) -> i64 {
        self.0
    }

    /// (année, mois, jour).
    pub fn ymd(&self) -> (i64, u32, u32) {
        let z = self.0 + 719468;
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = z - era * 146097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
        let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
        (if m <= 2 { y + 1 } else { y }, m, d)
    }

    pub fn ymd_string(&self) -> String {
        let (y, m, d) = self.ymd();
        format!("{y:04}-{m:02}-{d:02}")
    }

    /// Jour de la semaine : 0 = dimanche … 6 = samedi.
    pub fn weekday(&self) -> u32 {
        (((self.0 % 7) + 4 + 7) % 7) as u32
    }

    /// Nom court du mois (fr) : "janv." … "déc.".
    pub fn month_short(&self) -> &'static str {
        const NAMES: [&str; 12] = [
            "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
            "nov.", "déc.",
        ];
        let (_, m, _) = self.ymd();
        NAMES[(m as usize - 1).min(11)]
    }
}

/// Formate un nombre de tokens de façon compacte : `812`, `15,2 k`, `3,4 M`.
pub fn fmt_tokens(n: u64) -> String {
    let x = n as f64;
    if x < 1_000.0 {
        format!("{n}")
    } else if x < 1_000_000.0 {
        format!("{:.1} k", x / 1_000.0).replace('.', ",")
    } else if x < 1_000_000_000.0 {
        format!("{:.1} M", x / 1_000_000.0).replace('.', ",")
    } else {
        format!("{:.2} Md", x / 1_000_000_000.0).replace('.', ",")
    }
}

/// Formate un montant USD : `$0.0123` sous le cent, `$12.34` sinon.
pub fn fmt_usd(v: f64) -> String {
    if v > 0.0 && v < 0.01 {
        format!("${v:.4}")
    } else {
        format!("${v:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usage_and_aggregates_by_model() {
        let jsonl = concat!(
            r#"{"type":"user","message":{"role":"user","content":"salut"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-01T10:00:00Z","message":{"id":"m1","model":"claude-opus-4-8","usage":{"input_tokens":10,"output_tokens":100,"cache_creation_input_tokens":500,"cache_read_input_tokens":2000}}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-07-02T09:00:00Z","message":{"id":"m2","model":"claude-sonnet-5","usage":{"input_tokens":5,"output_tokens":50}}}"#,
        );
        let u = usage_from_jsonl(jsonl);
        assert_eq!(u.assistant_messages(), 2);
        let t = u.totals();
        assert_eq!(t.input, 15);
        assert_eq!(t.output, 150);
        assert_eq!(t.cache_creation, 500);
        assert_eq!(t.cache_read, 2000);

        let rows = u.model_rows();
        assert_eq!(rows.len(), 2);
        // Opus a le plus de tokens → en tête.
        assert_eq!(rows[0].model, "claude-opus-4-8");
        assert_eq!(rows[0].messages, 1);
        assert!(rows[0].cost.is_some());
    }

    #[test]
    fn deduplicates_repeated_message_ids() {
        let line = r#"{"type":"assistant","timestamp":"2026-07-01T10:00:00Z","message":{"id":"dup","model":"claude-opus-4-8","usage":{"input_tokens":10,"output_tokens":20}}}"#;
        let jsonl = format!("{line}\n{line}");
        let u = usage_from_jsonl(&jsonl);
        assert_eq!(u.assistant_messages(), 1);
        assert_eq!(u.totals().input, 10);
    }

    #[test]
    fn skips_lines_without_usage_or_zero() {
        let jsonl = concat!(
            r#"{"type":"assistant","message":{"model":"x","content":[]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"id":"z","model":"claude-opus-4-8","usage":{"input_tokens":0,"output_tokens":0}}}"#,
            "\n",
            "pas du json",
        );
        let u = usage_from_jsonl(jsonl);
        assert!(u.is_empty());
    }

    #[test]
    fn cost_uses_cache_multipliers() {
        // Opus 4.8 : 5/25 par million ; cache write 1,25×, cache read 0,1×.
        let p = price_for("claude-opus-4-8").unwrap();
        let t = Tokens {
            input: 1_000_000,
            output: 1_000_000,
            cache_creation: 1_000_000,
            cache_read: 1_000_000,
        };
        // 5 + 25 + 5*1.25 + 5*0.1 = 5 + 25 + 6.25 + 0.5 = 36.75
        assert!((p.cost(t) - 36.75).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_is_unpriced_but_counted() {
        let jsonl = r#"{"type":"assistant","message":{"id":"s","model":"<synthetic>","usage":{"input_tokens":1,"output_tokens":1}}}"#;
        let u = usage_from_jsonl(jsonl);
        assert!(u.has_unpriced());
        assert_eq!(u.total_cost(), 0.0);
        assert_eq!(u.model_rows()[0].cost, None);
    }

    #[test]
    fn price_families() {
        assert_eq!(price_for("claude-fable-5").unwrap().input, 10.0);
        assert_eq!(price_for("claude-opus-4-8").unwrap().input, 5.0);
        assert_eq!(price_for("claude-opus-4-1").unwrap().input, 15.0);
        assert_eq!(price_for("claude-3-opus-20240229").unwrap().input, 15.0);
        assert_eq!(price_for("claude-sonnet-5").unwrap().input, 3.0);
        assert_eq!(price_for("claude-3-5-sonnet-20241022").unwrap().input, 3.0);
        assert_eq!(price_for("claude-haiku-4-5").unwrap().input, 1.0);
        assert_eq!(price_for("claude-3-5-haiku-20241022").unwrap().input, 0.80);
        assert!(price_for("gpt-4").is_none());
    }

    #[test]
    fn civil_date_roundtrip_and_weekday() {
        // 1970-01-01 = jour 0, un jeudi (weekday 4).
        let epoch = Date::from_ymd(1970, 1, 1);
        assert_eq!(epoch.days(), 0);
        assert_eq!(epoch.weekday(), 4);
        assert_eq!(epoch.ymd(), (1970, 1, 1));

        // 2026-07-23 est un jeudi.
        let d = Date::from_ymd(2026, 7, 23);
        assert_eq!(d.ymd_string(), "2026-07-23");
        assert_eq!(d.weekday(), 4);
        // round-trip via jours.
        assert_eq!(Date::from_days(d.days()).ymd(), (2026, 7, 23));
        // le 1er janvier 2000 était un samedi.
        assert_eq!(Date::from_ymd(2000, 1, 1).weekday(), 6);
    }

    #[test]
    fn heatmap_places_activity_and_levels() {
        let mut u = Usage::default();
        u.record(
            "claude-opus-4-8",
            Some("2026-07-20T10:00:00Z"),
            Tokens {
                input: 1000,
                ..Default::default()
            },
        );
        u.record(
            "claude-opus-4-8",
            Some("2026-07-23T10:00:00Z"),
            Tokens {
                input: 100,
                ..Default::default()
            },
        );
        let anchor = Date::from_ymd(2026, 7, 23);
        let hm = u.heatmap(anchor, 4);
        assert_eq!(hm.weeks.len(), 4);
        assert_eq!(hm.max, 1000);

        // Retrouve les deux jours actifs et vérifie leurs niveaux.
        let mut found = 0;
        for col in &hm.weeks {
            for cell in col {
                if cell.date == "2026-07-20" {
                    assert_eq!(cell.tokens, 1000);
                    assert_eq!(cell.level, 4);
                    found += 1;
                }
                if cell.date == "2026-07-23" {
                    assert_eq!(cell.tokens, 100);
                    assert_eq!(cell.level, 1);
                    assert!(cell.in_range);
                    found += 1;
                }
            }
        }
        assert_eq!(found, 2);
    }

    #[test]
    fn merge_combines_two_usages() {
        let a = usage_from_jsonl(
            r#"{"type":"assistant","timestamp":"2026-07-01T10:00:00Z","message":{"id":"a","model":"claude-opus-4-8","usage":{"input_tokens":10,"output_tokens":20}}}"#,
        );
        let b = usage_from_jsonl(
            r#"{"type":"assistant","timestamp":"2026-07-01T11:00:00Z","message":{"id":"b","model":"claude-opus-4-8","usage":{"input_tokens":5,"output_tokens":5}}}"#,
        );
        let mut m = Usage::default();
        m.merge(&a);
        m.merge(&b);
        assert_eq!(m.assistant_messages(), 2);
        assert_eq!(m.totals().input, 15);
        assert_eq!(m.day_total("2026-07-01"), 40);
    }

    #[test]
    fn fmt_helpers() {
        assert_eq!(fmt_tokens(812), "812");
        assert_eq!(fmt_tokens(15_200), "15,2 k");
        assert_eq!(fmt_tokens(3_400_000), "3,4 M");
        assert_eq!(fmt_usd(12.3456), "$12.35");
        assert_eq!(fmt_usd(0.0012), "$0.0012");
    }
}
