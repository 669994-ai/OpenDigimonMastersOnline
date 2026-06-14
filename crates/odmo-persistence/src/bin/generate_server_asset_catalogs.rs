use std::{collections::HashMap, env, fs, path::{Path, PathBuf}};

use anyhow::{Context, Result, bail};
use odmo_types::{EvolutionAsset, EvolutionLineAsset, EvolutionStageAsset, ItemAsset};
use roxmltree::Document;
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Clone)]
struct Config {
    evolution_json: PathBuf,
    item_csv: PathBuf,
    item_str_xml: Option<PathBuf>,
    cooltime_xml: Option<PathBuf>,
    output_dir: PathBuf,
    database_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EvolutionExport {
    rows: Vec<EvolutionRow>,
}

#[derive(Debug, Deserialize)]
struct EvolutionRow {
    digimon_id: u32,
    evolutions: Vec<EvolutionEntry>,
}

#[derive(Debug, Deserialize)]
struct EvolutionEntry {
    evo_slot_or_index: u16,
    field_a: u32,
    field_b: u16,
    list1: Vec<EvolutionTargetInfo>,
    block1: EvolutionTargetInfo,
    field_c: u8,
    block2: EvolutionSlotOpenLimit,
    field_d: u16,
    pair: EvolutionUiIconPos,
    field_e: u32,
    block5: EvolutionJogressInfo,
}

#[derive(Debug, Deserialize)]
struct EvolutionTargetInfo {
    field_a: u16,
    field_b: u16,
    field_c: u32,
}

#[derive(Debug, Deserialize)]
struct EvolutionSlotOpenLimit {
    field_0: u16,
    field_1: u16,
    field_2: u32,
    field_3: u32,
    field_4: u16,
    field_5: u32,
    field_6: u16,
    field_7: u16,
    field_8: u16,
}

#[derive(Debug, Deserialize)]
struct EvolutionUiIconPos {
    field_a: u32,
    field_b: u32,
}

#[derive(Debug, Deserialize)]
struct EvolutionJogressInfo {
    field_0: u32,
    u16_values: Vec<u16>,
    #[serde(default)]
    list_values: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct ItemCsvRow {
    map_key: u32,
    #[serde(rename = "dwItemIDX")]
    item_id: u32,
    #[serde(rename = "dwIconID")]
    skill_code: u32,
    #[serde(rename = "wUnknown18")]
    item_type: u16,
    #[serde(rename = "dwUnknown20")]
    section: u32,
    #[serde(rename = "wUnknown28")]
    use_time_group: u16,
    #[serde(rename = "wUnknown2A")]
    overlap: u16,
    #[serde(rename = "sub2C_0")]
    tamer_min_level: u16,
    #[serde(rename = "sub2C_1")]
    tamer_max_level: u16,
    #[serde(rename = "sub34_0")]
    digimon_min_level: u16,
    #[serde(rename = "sub34_1")]
    digimon_max_level: u16,
    #[serde(rename = "wUnknown40")]
    use_character: u16,
    #[serde(rename = "dwUnknown44")]
    event_price_id: u32,
    #[serde(rename = "wUnknown48")]
    digicore_price: u16,
    #[serde(rename = "wUnknown4A")]
    event_price_amount: u16,
    #[serde(rename = "dwUnknown4C")]
    scan_price: u32,
    #[serde(rename = "dwUnknown50")]
    sell_price: u32,
    #[serde(rename = "wUnknown64")]
    bound_type: u16,
    value68_json: String,
    #[serde(rename = "dwUnknown78")]
    quest_require: u32,
    #[serde(rename = "bUnknown7C")]
    use_time_type: u8,
    #[serde(rename = "dwUnknown80")]
    usage_time_minutes: u32,
    #[serde(rename = "wUnknown86")]
    do_not_use_type: u16,
    #[serde(rename = "bUnknown88")]
    use_battle: u8,
}

fn parse_args() -> Result<Config> {
    let mut evolution_json = None;
    let mut item_csv = None;
    let mut item_str_xml = None;
    let mut cooltime_xml = None;
    let mut output_dir = PathBuf::from("data/server-assets");
    let mut database_url = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--evolution-json" => {
                evolution_json = Some(PathBuf::from(expect_value(&mut args, &arg)?))
            }
            "--item-csv" => item_csv = Some(PathBuf::from(expect_value(&mut args, &arg)?)),
            "--item-str-xml" => item_str_xml = Some(PathBuf::from(expect_value(&mut args, &arg)?)),
            "--cooltime-xml" => cooltime_xml = Some(PathBuf::from(expect_value(&mut args, &arg)?)),
            "--output-dir" => output_dir = PathBuf::from(expect_value(&mut args, &arg)?),
            "--database-url" => database_url = Some(expect_value(&mut args, &arg)?),
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    Ok(Config {
        evolution_json: evolution_json.context("--evolution-json is required")?,
        item_csv: item_csv.context("--item-csv is required")?,
        item_str_xml,
        cooltime_xml,
        output_dir,
        database_url,
    })
}

fn expect_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("missing value for {flag}"))
}

fn print_help() {
    println!(
        "generate_server_asset_catalogs\n\
         \n\
         Required:\n\
           --evolution-json <PATH>\n\
           --item-csv <PATH>\n\
         Optional:\n\
           --item-str-xml <PATH>\n\
           --cooltime-xml <PATH>\n\
           --output-dir <DIR>        (default: data/server-assets)\n\
           --database-url <URL>      sync generated catalogs into PostgreSQL\n"
    );
}

fn parse_item_names(path: &Path) -> Result<HashMap<u32, String>> {
    let xml = fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    let doc = Document::parse(&xml)
        .with_context(|| format!("failed to parse '{}'", path.display()))?;
    let mut names = HashMap::new();

    for record in doc.descendants().filter(|node| node.has_tag_name("Record")) {
        let Some(id_text) = record.attribute("id0") else {
            continue;
        };
        let Ok(text_key) = id_text.parse::<u32>() else {
            continue;
        };

        let mut values = record
            .children()
            .filter(|node| node.is_element() && node.tag_name().name().starts_with("string"))
            .map(|node| node.text().unwrap_or("").trim().to_string());

        let primary = values.nth(1).unwrap_or_default();
        let fallback = record
            .children()
            .find(|node| node.has_tag_name("string0"))
            .and_then(|node| node.text())
            .unwrap_or("")
            .trim()
            .to_string();

        let chosen = if primary.is_empty() { fallback } else { primary };
        if !chosen.is_empty() {
            names.insert(text_key, chosen);
        }
    }

    Ok(names)
}

fn parse_cooltimes(path: &Path) -> Result<HashMap<u16, i32>> {
    let xml = fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    let doc = Document::parse(&xml)
        .with_context(|| format!("failed to parse '{}'", path.display()))?;
    let mut groups = HashMap::new();

    for record in doc.descendants().filter(|node| node.has_tag_name("Record")) {
        let Some(hex) = record
            .children()
            .find(|node| node.has_tag_name("hex"))
            .and_then(|node| node.text())
        else {
            continue;
        };

        let bytes = decode_hex(hex.trim())?;
        if bytes.len() != 13 {
            continue;
        }

        let group_id = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as u16;
        let seconds = f32::from_le_bytes(bytes[9..13].try_into().unwrap());
        groups.insert(group_id, seconds.round() as i32);
    }

    Ok(groups)
}

fn decode_hex(hex: &str) -> Result<Vec<u8>> {
    let compact: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() % 2 != 0 {
        bail!("odd hex length");
    }
    let mut bytes = Vec::with_capacity(compact.len() / 2);
    for index in (0..compact.len()).step_by(2) {
        let byte = u8::from_str_radix(&compact[index..index + 2], 16)
            .with_context(|| format!("invalid hex byte at index {index}"))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn parse_item_assets(
    item_csv: &Path,
    item_names: &HashMap<u32, String>,
    cooltimes: &HashMap<u16, i32>,
) -> Result<Vec<ItemAsset>> {
    let mut reader = csv::Reader::from_path(item_csv)
        .with_context(|| format!("failed to read '{}'", item_csv.display()))?;
    let mut assets = Vec::new();

    for row in reader.deserialize::<ItemCsvRow>() {
        let row = row.with_context(|| format!("failed to decode '{}'", item_csv.display()))?;
        let mut quest_requirements =
            serde_json::from_str::<Vec<i32>>(&row.value68_json).unwrap_or_default();
        if row.quest_require > 0 {
            quest_requirements.push(row.quest_require as i32);
        }

        let name = item_names
            .get(&row.map_key)
            .or_else(|| item_names.get(&row.item_id))
            .cloned()
            .unwrap_or_default();

        assets.push(ItemAsset {
            item_id: row.item_id as i32,
            name,
            item_type: i32::from(row.item_type),
            section: row.section as i32,
            combined_section: i32::from(row.item_type) * 1000 + row.section as i32,
            overlap: row.overlap as i16,
            use_time_group: i32::from(row.use_time_group),
            use_time_seconds: cooltimes.get(&row.use_time_group).copied().unwrap_or_default(),
            quest_requirements,
            use_character: i32::from(row.use_character),
            bound_type: i32::from(row.bound_type),
            use_time_type: i32::from(row.use_time_type),
            skill_code: i64::from(row.skill_code),
            tamer_min_level: row.tamer_min_level.min(u16::from(u8::MAX)) as u8,
            tamer_max_level: row.tamer_max_level.min(u16::from(u8::MAX)) as u8,
            digimon_min_level: row.digimon_min_level.min(u16::from(u8::MAX)) as u8,
            digimon_max_level: row.digimon_max_level.min(u16::from(u8::MAX)) as u8,
            sell_price: i64::from(row.sell_price),
            scan_price: row.scan_price as i32,
            digicore_price: i32::from(row.digicore_price),
            event_price_id: row.event_price_id as i32,
            event_price_amount: i32::from(row.event_price_amount),
            usage_time_minutes: row.usage_time_minutes as i32,
            do_not_use_type: i32::from(row.do_not_use_type),
            use_battle: i32::from(row.use_battle),
        });
    }

    assets.sort_by_key(|asset| asset.item_id);
    Ok(assets)
}

fn map_stage(target: &EvolutionTargetInfo) -> EvolutionStageAsset {
    EvolutionStageAsset {
        target_type: target.field_c as i32,
        value: (i32::from(target.field_b) << 16) | i32::from(target.field_a),
    }
}

fn parse_evolution_assets(path: &Path) -> Result<Vec<EvolutionAsset>> {
    let payload = fs::read_to_string(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    let export: EvolutionExport = serde_json::from_str(&payload)
        .with_context(|| format!("failed to parse '{}'", path.display()))?;

    let mut assets = Vec::with_capacity(export.rows.len());
    for row in export.rows {
        let mut lines = Vec::with_capacity(row.evolutions.len());
        for entry in row.evolutions {
            let mut stages = entry.list1.iter().map(map_stage).collect::<Vec<_>>();
            if entry.block1.field_c != 0 {
                stages.push(map_stage(&entry.block1));
            }

            lines.push(EvolutionLineAsset {
                type_id: entry.field_a as i32,
                slot_level: entry.evo_slot_or_index.min(u16::from(u8::MAX)) as u8,
                unlock_level: entry.block2.field_1.min(u16::from(u8::MAX)) as u8,
                unlock_quest_id: entry.block2.field_2 as i32,
                unlock_item_section: entry.block2.field_3 as i32,
                unlock_item_section_amount: i32::from(entry.block2.field_4),
                required_item: entry.block2.field_5 as i32,
                required_amount: i32::from(entry.block2.field_6),
                required_ds: i32::from(entry.field_d),
                enabled: entry.field_c,
                open_qualification: entry.block2.field_0,
                use_item_hint: i32::from(entry.field_b) as u16,
                required_intimacy: entry.block2.field_7,
                open_crest: entry.block2.field_8,
                evolution_tree: entry.field_e as i32,
                icon_pos_x: entry.pair.field_a as i32,
                icon_pos_y: entry.pair.field_b as i32,
                jogress_quest_check: entry.block5.field_0 as i32,
                jogress_chipset_type: entry
                    .block5
                    .u16_values
                    .first()
                    .copied()
                    .unwrap_or_default() as i32,
                jogress_consumable_chipset_type: entry
                    .block5
                    .u16_values
                    .get(1)
                    .copied()
                    .unwrap_or_default() as i32,
                jogress_chipset_amount: entry
                    .block5
                    .u16_values
                    .get(2)
                    .copied()
                    .unwrap_or_default() as i32,
                jogress_period_chipset_type: entry
                    .block5
                    .u16_values
                    .get(3)
                    .copied()
                    .unwrap_or_default() as i32,
                jogress_need_digimon_types: entry
                    .block5
                    .list_values
                    .iter()
                    .map(|value| *value as i32)
                    .collect(),
                stages,
            });
        }

        assets.push(EvolutionAsset {
            base_type: row.digimon_id as i32,
            lines,
        });
    }

    assets.sort_by_key(|asset| asset.base_type);
    Ok(assets)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(value)?;
    fs::write(path, payload).with_context(|| format!("failed to write '{}'", path.display()))?;
    Ok(())
}

async fn sync_postgres(
    database_url: &str,
    evolution_assets: &[EvolutionAsset],
    item_assets: &[ItemAsset],
) -> Result<()> {
    let pool = PgPool::connect(database_url).await?;
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM evolution_assets")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM item_assets")
        .execute(&mut *tx)
        .await?;

    for asset in evolution_assets {
        sqlx::query(
            "INSERT INTO evolution_assets (base_type, payload) VALUES ($1, $2)
             ON CONFLICT (base_type) DO UPDATE SET payload = EXCLUDED.payload",
        )
        .bind(asset.base_type)
        .bind(serde_json::to_value(asset)?)
        .execute(&mut *tx)
        .await?;
    }

    for asset in item_assets {
        sqlx::query(
            "INSERT INTO item_assets (item_id, payload) VALUES ($1, $2)
             ON CONFLICT (item_id) DO UPDATE SET payload = EXCLUDED.payload",
        )
        .bind(asset.item_id)
        .bind(serde_json::to_value(asset)?)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = parse_args()?;

    let item_names = match &config.item_str_xml {
        Some(path) => parse_item_names(path)?,
        None => HashMap::new(),
    };
    let cooltimes = match &config.cooltime_xml {
        Some(path) => parse_cooltimes(path)?,
        None => HashMap::new(),
    };

    let evolution_assets = parse_evolution_assets(&config.evolution_json)?;
    let item_assets = parse_item_assets(&config.item_csv, &item_names, &cooltimes)?;

    let evolution_out = config.output_dir.join("evolution_assets.json");
    let item_out = config.output_dir.join("item_assets.json");
    write_json(&evolution_out, &evolution_assets)?;
    write_json(&item_out, &item_assets)?;

    if let Some(database_url) = &config.database_url {
        sync_postgres(database_url, &evolution_assets, &item_assets).await?;
    }

    println!(
        "generated {} evolution assets and {} item assets",
        evolution_assets.len(),
        item_assets.len()
    );
    println!("wrote {}", evolution_out.display());
    println!("wrote {}", item_out.display());
    if config.database_url.is_some() {
        println!("postgres sync completed");
    }

    Ok(())
}
