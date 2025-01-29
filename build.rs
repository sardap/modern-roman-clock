use std::{collections::HashMap, env, fs, path::Path};

use chrono::{Datelike, NaiveDate};
use convert_case::{Case, Casing};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct YearOwner {
    pub start: String,
    pub owner: String,
}

impl YearOwner {
    pub fn start(&self) -> NaiveDate {
        NaiveDate::parse_from_str(&self.start, "%Y-%m-%d").unwrap()
    }
}


fn parse_rulers() -> HashMap<String, Vec<YearOwner>> {
    let reader = include_str!("rulers.json");
    return serde_json::from_str(&reader).unwrap()
}


fn generate_ruler_map(prefix: &str, rulers_chart: &[YearOwner]) -> String {
    let prefix = prefix.to_uppercase();
    let mut contents = String::new();
    // Whoever was ruler on the first day of the year it's there year
    let mut year_map = HashMap::new();

    for i in 0..rulers_chart.len() {
        let owner = rulers_chart.get(i).unwrap();

        let owner_start = owner.start();

        let start = if i >= 1 {
            if owner_start.month() == 1 && owner_start.day() == 1 {
                owner_start.year()
            } else {
                owner_start.year() + 1
            }
        } else {
            i32::MIN
        };

        let end = match rulers_chart.get(i + 1) {
            Some(next) => {
                let next_start = next.start();

                if next_start.month() == 1 && next_start.day() == 1 {
                    next_start.year() - 1
                } else {
                    next_start.year()
                }
            }
            None => i32::MAX,
        };

        if start > end {
            continue;
        }

        let entry = year_map.entry(owner.owner.clone()).or_insert_with(Vec::new);
        entry.push((start, end));
    }

    let create_owner_name = |owner: &str| -> String {
        format!("{}_{}", prefix, owner.replace(".", "_").to_case(Case::UpperSnake))
    };

    let mut rulers = year_map.keys().map(|i| i).collect::<Vec<_>>();
    rulers.sort_by(|a, b| year_map[a.as_str()][0].0.cmp(&year_map[b.as_str()][0].0));
    for owner in &rulers {
        // create years
        let owner_years_const_name = create_owner_name(owner);
        contents.push_str(&format!(
            "const {}: [(i32, i32); {}] = [",
            owner_years_const_name,
            year_map[owner.as_str()].len()
        ));
        for year in year_map[owner.as_str()].iter() {
            contents.push_str(&format!("({}, {}), ", year.0, year.1));
        }
        contents.push_str("];\n");
    }

    contents.push_str(&format!(
        "pub const {}_RULER_CHART: [YearOwner; {}] = [",
        prefix,
        rulers.len()
    ));
    for owner in rulers.iter() {
        let owner_years_const_name = create_owner_name(owner);
        contents.push_str(&format!(
            "YearOwner {{ owner: \"{}\", years: &{} }},\n",
            owner, owner_years_const_name
        ));
    }
    contents.push_str("];");

    contents
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("year_owner.rs");
    let mut contents = String::new();

    contents.push_str(
        "pub struct YearOwner {
            pub owner: &'static str,
            pub years: &'static [(i32, i32)],
        }
        ",
    );

    let rulers = parse_rulers();

    for (prefix, rulers_chart) in rulers.iter() {
        contents.push_str(&generate_ruler_map(prefix, rulers_chart));
    }

    contents.push_str(
        "pub fn get_owners_for_country(country_iso_3166: &str) -> Option<&'static [YearOwner]> {
            match country_iso_3166 {
        ",
    );

    for (prefix, _) in rulers.iter() {
        contents.push_str(&format!(
            "\"{}\" => Some(&{}_RULER_CHART),\n",
            prefix.to_uppercase(),
            prefix.to_uppercase()
        ));
    }

    contents.push_str(
        "_ => None,
            }
        }",
    );

    fs::write(&dest_path, contents).unwrap();
}
