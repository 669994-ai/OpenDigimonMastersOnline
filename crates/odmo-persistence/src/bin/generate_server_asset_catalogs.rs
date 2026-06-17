use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use odmo_types::{
    DigimonAsset, EvolutionAsset, EvolutionLineAsset, EvolutionStageAsset, ItemAsset,
};
use roxmltree::Document;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[derive(Debug, Clone)]
struct Config {
    evolution_json: Option<PathBuf>,
    item_csv: Option<PathBuf>,
    digimon_pack_source: Option<DigimonPackSource>,
    item_str_xml: Option<PathBuf>,
    cooltime_xml: Option<PathBuf>,
    output_dir: PathBuf,
    database_url: Option<String>,
}

#[derive(Debug, Clone)]
struct DigimonPackSource {
    pack_dir: PathBuf,
    toolkit_dir: PathBuf,
}

struct DecodedPack03Workspace {
    temp_root: PathBuf,
    pack03_hf: PathBuf,
    pack03_pf: PathBuf,
    decoded_dir: PathBuf,
}

impl Drop for DecodedPack03Workspace {
    fn drop(&mut self) {
        if self.temp_root.exists() {
            let _ = fs::remove_dir_all(&self.temp_root);
        }
    }
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
    #[serde(rename = "sub70_u8_0")]
    digivice_skill_slots: u8,
    #[serde(rename = "sub70_u8_1")]
    digivice_chipset_slots: u8,
    #[serde(rename = "bUnknown7C")]
    use_time_type: u8,
    #[serde(rename = "dwUnknown80")]
    usage_time_minutes: u32,
    #[serde(rename = "wUnknown86")]
    do_not_use_type: u16,
    #[serde(rename = "bUnknown88")]
    use_battle: u8,
}

#[derive(Debug, Serialize)]
struct DigimonAssetSourceManifest {
    source: &'static str,
    import_method: &'static str,
    toolkit: &'static str,
    pack_files: Vec<PackFileFingerprint>,
    digimon_payload_path: String,
    digimon_payload_sha256: String,
}

#[derive(Debug, Serialize)]
struct ItemAssetSourceManifest {
    source: &'static str,
    import_method: &'static str,
    toolkit: &'static str,
    pack_files: Vec<PackFileFingerprint>,
    item_payload_path: String,
    item_payload_sha256: String,
    cooltime_payload_path: String,
    cooltime_payload_sha256: String,
    item_name_source: &'static str,
}

#[derive(Debug, Serialize)]
struct PackFileFingerprint {
    name: String,
    sha256: String,
}

fn parse_args() -> Result<Config> {
    let mut evolution_json = None;
    let mut item_csv = None;
    let mut modern_pack_dir = None;
    let mut pack_toolkit_dir = None;
    let mut item_str_xml = None;
    let mut cooltime_xml = None;
    let mut allow_noncanonical_reverse_inputs = false;
    let mut output_dir = PathBuf::from("data/server-assets");
    let mut database_url = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--evolution-json" => {
                evolution_json = Some(PathBuf::from(expect_value(&mut args, &arg)?))
            }
            "--item-csv" => item_csv = Some(PathBuf::from(expect_value(&mut args, &arg)?)),
            "--modern-pack-dir" => {
                modern_pack_dir = Some(PathBuf::from(expect_value(&mut args, &arg)?))
            }
            "--pack-toolkit-dir" => {
                pack_toolkit_dir = Some(PathBuf::from(expect_value(&mut args, &arg)?))
            }
            "--item-str-xml" => item_str_xml = Some(PathBuf::from(expect_value(&mut args, &arg)?)),
            "--cooltime-xml" => cooltime_xml = Some(PathBuf::from(expect_value(&mut args, &arg)?)),
            "--allow-noncanonical-reverse-inputs" => allow_noncanonical_reverse_inputs = true,
            "--output-dir" => output_dir = PathBuf::from(expect_value(&mut args, &arg)?),
            "--database-url" => database_url = Some(expect_value(&mut args, &arg)?),
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    let digimon_pack_source = match (modern_pack_dir, pack_toolkit_dir) {
        (Some(pack_dir), Some(toolkit_dir)) => Some(DigimonPackSource {
            pack_dir,
            toolkit_dir,
        }),
        (None, None) => None,
        _ => bail!("--modern-pack-dir and --pack-toolkit-dir must be provided together"),
    };

    if evolution_json.is_none() && item_csv.is_none() && digimon_pack_source.is_none() {
        bail!(
            "provide at least one data source: --evolution-json, --item-csv, or (--modern-pack-dir + --pack-toolkit-dir)"
        );
    }

    if !allow_noncanonical_reverse_inputs
        && (item_csv.is_some() || item_str_xml.is_some() || cooltime_xml.is_some())
    {
        bail!(
            "manual item CSV/XML inputs are non-canonical reverse helpers; rerun with --allow-noncanonical-reverse-inputs if you intentionally need them, otherwise refresh from the current Pack03 files"
        );
    }

    Ok(Config {
        evolution_json,
        item_csv,
        digimon_pack_source,
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
         Provide one or more data sources:\n\
          --evolution-json <PATH>    exported DEvolutionList JSON for evolution rules\n\
          --item-csv <PATH>          exported ItemData CSV for item rules (non-canonical reverse helper)\n\
          --modern-pack-dir <DIR>    current modern client Data pack directory\n\
          --pack-toolkit-dir <DIR>   DmoPackToolkit workspace used to derive DigimonListData\n\
         Optional helpers:\n\
          --item-str-xml <PATH>\n\
          --cooltime-xml <PATH>\n\
          --allow-noncanonical-reverse-inputs\n\
           --output-dir <DIR>         (default: data/server-assets)\n\
           --database-url <URL>       sync only the generated catalogs into PostgreSQL\n\
         \n\
         Notes:\n\
           - Server catalogs are runtime-owned by this project.\n\
           - Item and Digimon metadata should be regenerated from the current packs through DmoPackToolkit.\n\
           - Exported CSV/XML helpers remain opt-in reverse aids, not the canonical refresh flow.\n\
           - The shipped server must read only project-owned catalogs/DB rows, never extracted pack payloads.\n"
    );
}

fn parse_item_names(path: &Path) -> Result<HashMap<u32, String>> {
    let xml =
        fs::read_to_string(path).with_context(|| format!("failed to read '{}'", path.display()))?;
    let doc =
        Document::parse(&xml).with_context(|| format!("failed to parse '{}'", path.display()))?;
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

        let chosen = if primary.is_empty() {
            fallback
        } else {
            primary
        };
        if !chosen.is_empty() {
            names.insert(text_key, chosen);
        }
    }

    Ok(names)
}

fn parse_cooltimes(path: &Path) -> Result<HashMap<u16, i32>> {
    let xml =
        fs::read_to_string(path).with_context(|| format!("failed to read '{}'", path.display()))?;
    let doc =
        Document::parse(&xml).with_context(|| format!("failed to parse '{}'", path.display()))?;
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
    if !compact.len().is_multiple_of(2) {
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
            use_time_seconds: cooltimes
                .get(&row.use_time_group)
                .copied()
                .unwrap_or_default(),
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
            digivice_skill_slots: row.digivice_skill_slots,
            digivice_chipset_slots: row.digivice_chipset_slots,
            usage_time_minutes: row.usage_time_minutes as i32,
            do_not_use_type: i32::from(row.do_not_use_type),
            use_battle: i32::from(row.use_battle),
        });
    }

    assets.sort_by_key(|asset| asset.item_id);
    Ok(assets)
}

fn load_existing_item_names(path: &Path) -> Result<HashMap<u32, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let payload =
        fs::read_to_string(path).with_context(|| format!("failed to read '{}'", path.display()))?;
    let assets: Vec<ItemAsset> = serde_json::from_str(&payload)
        .with_context(|| format!("failed to parse '{}'", path.display()))?;
    Ok(assets
        .into_iter()
        .filter(|asset| !asset.name.is_empty())
        .map(|asset| (asset.item_id as u32, asset.name))
        .collect())
}

fn map_stage(target: &EvolutionTargetInfo) -> EvolutionStageAsset {
    EvolutionStageAsset {
        target_type: target.field_c as i32,
        value: (i32::from(target.field_b) << 16) | i32::from(target.field_a),
    }
}

fn parse_evolution_assets(path: &Path) -> Result<Vec<EvolutionAsset>> {
    let payload =
        fs::read_to_string(path).with_context(|| format!("failed to read '{}'", path.display()))?;
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
                jogress_chipset_type: entry.block5.u16_values.first().copied().unwrap_or_default()
                    as i32,
                jogress_consumable_chipset_type: entry
                    .block5
                    .u16_values
                    .get(1)
                    .copied()
                    .unwrap_or_default() as i32,
                jogress_chipset_amount: entry.block5.u16_values.get(2).copied().unwrap_or_default()
                    as i32,
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

fn resolve_pack_file(pack_dir: &Path, canonical_name: &str) -> Result<PathBuf> {
    let candidates = [
        canonical_name.to_string(),
        canonical_name.to_ascii_lowercase(),
        canonical_name.to_ascii_uppercase(),
    ];
    for candidate in candidates {
        let path = pack_dir.join(&candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    bail!(
        "could not find '{}' under '{}'",
        canonical_name,
        pack_dir.display()
    );
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read '{}'", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn run_command(command: &mut Command, description: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to start {description}"))?;
    if !status.success() {
        bail!("{description} failed with status {status}");
    }
    Ok(())
}

fn decode_pack03_workspace(source: &DigimonPackSource) -> Result<DecodedPack03Workspace> {
    let pack03_hf = resolve_pack_file(&source.pack_dir, "Pack03.hf")?;
    let pack03_pf = resolve_pack_file(&source.pack_dir, "Pack03.pf")?;
    let toolkit_project = source
        .toolkit_dir
        .join("src")
        .join("DmoDecryptor")
        .join("DmoDecryptor.csproj");
    if !toolkit_project.exists() {
        bail!(
            "could not find DmoPackToolkit project at '{}'",
            toolkit_project.display()
        );
    }

    let temp_root = env::temp_dir().join(format!("odmo-pack03-import-{}", std::process::id()));
    if temp_root.exists() {
        let _ = fs::remove_dir_all(&temp_root);
    }
    fs::create_dir_all(&temp_root)
        .with_context(|| format!("failed to create '{}'", temp_root.display()))?;

    let extracted_dir = temp_root.join("pack03-extracted");
    let decoded_dir = temp_root.join("pack03-decoded");

    run_command(
        Command::new("dotnet")
            .current_dir(&source.toolkit_dir)
            .arg("run")
            .arg("--project")
            .arg(&toolkit_project)
            .arg("--")
            .arg("extract")
            .arg(&pack03_hf)
            .arg(&extracted_dir),
        "DmoPackToolkit extract for Pack03",
    )?;

    run_command(
        Command::new("dotnet")
            .current_dir(&source.toolkit_dir)
            .arg("run")
            .arg("--project")
            .arg(&toolkit_project)
            .arg("--")
            .arg("decode-bin")
            .arg(&extracted_dir)
            .arg(&decoded_dir),
        "DmoPackToolkit decode-bin for Pack03",
    )?;

    Ok(DecodedPack03Workspace {
        temp_root,
        pack03_hf,
        pack03_pf,
        decoded_dir,
    })
}

fn find_decoded_table_file(
    workspace: &DecodedPack03Workspace,
    relative_path: &str,
) -> Result<PathBuf> {
    let exact = workspace.decoded_dir.join(relative_path);
    if exact.exists() {
        return Ok(exact);
    }

    bail!("could not find decoded Pack03 table '{}'", relative_path)
}

struct CursorReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> CursorReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u8(&mut self) -> Result<u8> {
        let Some(&value) = self.bytes.get(self.offset) else {
            bail!("unexpected EOF reading u8 at {}", self.offset);
        };
        self.offset += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16> {
        let end = self.offset + 2;
        let slice = self
            .bytes
            .get(self.offset..end)
            .context("unexpected EOF reading u16")?;
        self.offset = end;
        Ok(u16::from_le_bytes(slice.try_into().unwrap()))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let end = self.offset + 4;
        let slice = self
            .bytes
            .get(self.offset..end)
            .context("unexpected EOF reading u32")?;
        self.offset = end;
        Ok(u32::from_le_bytes(slice.try_into().unwrap()))
    }

    fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    fn skip(&mut self, len: usize) -> Result<()> {
        let end = self.offset + len;
        if end > self.bytes.len() {
            bail!("unexpected EOF skipping {} bytes at {}", len, self.offset);
        }
        self.offset = end;
        Ok(())
    }

    fn read_lstring(&mut self) -> Result<()> {
        let len = self.read_u32()? as usize;
        self.skip(len)
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }
}

fn load_table_payload_bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("failed to read '{}'", path.display()))
}

fn parse_cooltimes_payload(path: &Path) -> Result<HashMap<u16, i32>> {
    let bytes = load_table_payload_bytes(path)?;
    let mut reader = CursorReader::new(&bytes);
    let count = reader.read_u32()? as usize;
    let mut groups = HashMap::with_capacity(count);

    for _ in 0..count {
        let group_id = reader.read_u32()? as u16;
        reader.skip(5)?;
        let seconds = reader.read_f32()?;
        groups.insert(group_id, seconds.round() as i32);
    }

    Ok(groups)
}

fn parse_item_assets_payload(
    path: &Path,
    item_names: &HashMap<u32, String>,
    cooltimes: &HashMap<u16, i32>,
) -> Result<Vec<ItemAsset>> {
    let bytes = load_table_payload_bytes(path)?;
    let mut reader = CursorReader::new(&bytes);
    let count = reader.read_u32()? as usize;
    let mut assets = Vec::with_capacity(count);

    for _ in 0..count {
        let map_key = reader.read_u32()?;
        let item_id = reader.read_u32()?;
        let _name_idx = reader.read_u32()?;
        let _b_type = reader.read_u8()?;
        let skill_code = reader.read_u32()?;
        let _b_unknown_14 = reader.read_u8()?;
        let _w_unknown_16 = reader.read_u16()?;
        let item_type = reader.read_u16()?;
        let _w_unknown_1a = reader.read_u16()?;
        let _dw_unknown_1c = reader.read_u32()?;
        let section = reader.read_u32()?;
        let _w_unknown_24 = reader.read_u16()?;
        let _b_unknown_26 = reader.read_u8()?;
        let use_time_group = reader.read_u16()?;
        let overlap = reader.read_u16()?;
        let tamer_min_level = reader.read_u16()?;
        let tamer_max_level = reader.read_u16()?;
        let digimon_min_level = reader.read_u16()?;
        let digimon_max_level = reader.read_u16()?;
        let _w_unknown_3c = reader.read_u16()?;
        let _w_unknown_3e = reader.read_u16()?;
        let use_character = reader.read_u16()?;
        let _w_unknown_42 = reader.read_u16()?;
        let event_price_id = reader.read_u32()?;
        let digicore_price = reader.read_u16()?;
        let event_price_amount = reader.read_u16()?;
        let scan_price = reader.read_u32()?;
        let sell_price = reader.read_u32()?;
        let _b_unknown_54 = reader.read_u8()?;
        let _b_unknown_55 = reader.read_u8()?;
        let _sub58_u16 = reader.read_u16()?;
        let _sub58_u8_0 = reader.read_u8()?;
        let _sub58_u8_1 = reader.read_u8()?;
        let _sub58_u8_2 = reader.read_u8()?;
        let bound_type = reader.read_u16()?;
        let _b_unknown_66 = reader.read_u8()?;

        let value_count = reader.read_u32()? as usize;
        let mut quest_requirements = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            quest_requirements.push(reader.read_u32()? as i32);
        }

        let digivice_skill_slots = reader.read_u8()?;
        let digivice_chipset_slots = reader.read_u8()?;
        let quest_require = reader.read_u32()?;
        if quest_require > 0 {
            quest_requirements.push(quest_require as i32);
        }
        let use_time_type = reader.read_u8()?;
        let usage_time_minutes = reader.read_u32()?;
        let _b_unknown_84 = reader.read_u8()?;
        let do_not_use_type = reader.read_u16()?;
        let use_battle = reader.read_u8()?;

        reader.read_lstring()?;
        reader.read_lstring()?;
        let string2_len = reader.read_u32()? as usize;
        if string2_len <= reader.remaining() {
            reader.skip(string2_len)?;
        }

        let name = item_names
            .get(&map_key)
            .or_else(|| item_names.get(&item_id))
            .cloned()
            .unwrap_or_default();

        assets.push(ItemAsset {
            item_id: item_id as i32,
            name,
            item_type: i32::from(item_type),
            section: section as i32,
            combined_section: i32::from(item_type) * 1000 + section as i32,
            overlap: overlap as i16,
            use_time_group: i32::from(use_time_group),
            use_time_seconds: cooltimes.get(&use_time_group).copied().unwrap_or_default(),
            quest_requirements,
            use_character: i32::from(use_character),
            bound_type: i32::from(bound_type),
            use_time_type: i32::from(use_time_type),
            skill_code: i64::from(skill_code),
            tamer_min_level: tamer_min_level.min(u16::from(u8::MAX)) as u8,
            tamer_max_level: tamer_max_level.min(u16::from(u8::MAX)) as u8,
            digimon_min_level: digimon_min_level.min(u16::from(u8::MAX)) as u8,
            digimon_max_level: digimon_max_level.min(u16::from(u8::MAX)) as u8,
            sell_price: i64::from(sell_price),
            scan_price: scan_price as i32,
            digicore_price: i32::from(digicore_price),
            event_price_id: event_price_id as i32,
            event_price_amount: i32::from(event_price_amount),
            digivice_skill_slots,
            digivice_chipset_slots,
            usage_time_minutes: usage_time_minutes as i32,
            do_not_use_type: i32::from(do_not_use_type),
            use_battle: i32::from(use_battle),
        });
    }

    assets.sort_by_key(|asset| asset.item_id);
    Ok(assets)
}

fn parse_digimon_assets_payload(path: &Path) -> Result<Vec<DigimonAsset>> {
    let bytes = load_table_payload_bytes(path)?;
    let mut reader = CursorReader::new(&bytes);
    let count = reader.read_u32()? as usize;
    let mut assets = Vec::with_capacity(count);

    for _ in 0..count {
        let _digimon_table_key = reader.read_u32()?;
        for _ in 0..10 {
            let _ = reader.read_u16()?;
        }
        let digimon_id = reader.read_u32()? as i32;
        let _model_id = reader.read_u32()?;
        let _select_scale = reader.read_f32()?;
        let base_level = reader.read_u32()? as i32;
        let _grow_type = reader.read_u16()?;
        let _field_p = reader.read_u32()?;
        let _char_size = reader.read_u16()?;
        let evolution_type = i32::from(reader.read_u8()?);
        let _field_s = reader.read_u16()?;
        let _attribute_type = reader.read_u8()?;

        let family_count = reader.read_u32()? as usize;
        for _ in 0..family_count {
            let _ = reader.read_u16()?;
        }

        let _base_nature_type = reader.read_u16()?;
        let base_nature_count = reader.read_u32()? as usize;
        for _ in 0..base_nature_count {
            let _ = reader.read_u16()?;
        }

        reader.read_lstring()?;
        reader.read_lstring()?;

        let skill_count = reader.read_u32()? as usize;
        for _ in 0..skill_count {
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
            let _ = reader.read_u32()?;
        }

        let _walk_len = reader.read_f32()?;
        let _run_len = reader.read_f32()?;
        let _a_run_len = reader.read_f32()?;
        let _digimon_rank = reader.read_u32()?;
        let _tail_u32 = reader.read_u32()?;

        assets.push(DigimonAsset {
            digimon_id,
            base_level,
            evolution_type,
        });
    }

    if reader.offset != bytes.len() {
        bail!(
            "digimon asset parser stopped at {}/{} bytes for '{}'",
            reader.offset,
            bytes.len(),
            path.display()
        );
    }

    assets.sort_by_key(|asset| asset.digimon_id);
    Ok(assets)
}

fn load_digimon_assets_from_decoded_pack03(
    workspace: &DecodedPack03Workspace,
) -> Result<(Vec<DigimonAsset>, DigimonAssetSourceManifest)> {
    let digimon_bin =
        find_decoded_table_file(workspace, "data/bin/table/DigimonListData.payload.bin")?;

    let assets = parse_digimon_assets_payload(&digimon_bin).with_context(|| {
        format!(
            "failed to parse '{}' as the normalized DigimonListData payload; the current pack schema may have drifted and needs renewed reverse evidence before this catalog can be regenerated safely",
            digimon_bin.display()
        )
    })?;
    let manifest = DigimonAssetSourceManifest {
        source: "modern-pack03-via-dmopacktoolkit",
        import_method: "extract-pack03-and-decode-digimonlistdata",
        toolkit: "DmoPackToolkit",
        pack_files: vec![
            PackFileFingerprint {
                name: workspace
                    .pack03_hf
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Pack03.hf")
                    .to_string(),
                sha256: sha256_file(&workspace.pack03_hf)?,
            },
            PackFileFingerprint {
                name: workspace
                    .pack03_pf
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Pack03.pf")
                    .to_string(),
                sha256: sha256_file(&workspace.pack03_pf)?,
            },
        ],
        digimon_payload_path: digimon_bin
            .strip_prefix(&workspace.temp_root)
            .unwrap_or(&digimon_bin)
            .display()
            .to_string(),
        digimon_payload_sha256: sha256_file(&digimon_bin)?,
    };

    Ok((assets, manifest))
}

fn load_item_assets_from_decoded_pack03(
    workspace: &DecodedPack03Workspace,
    existing_names: &HashMap<u32, String>,
    item_name_source: &'static str,
) -> Result<(Vec<ItemAsset>, ItemAssetSourceManifest)> {
    let item_payload = find_decoded_table_file(workspace, "data/bin/table/ItemData.payload.bin")?;
    let cooltime_payload =
        find_decoded_table_file(workspace, "data/bin/table/CoolTime.payload.bin")?;

    let assets = parse_item_assets_payload(
        &item_payload,
        existing_names,
        &parse_cooltimes_payload(&cooltime_payload)?,
    )?;
    let manifest = ItemAssetSourceManifest {
        source: "modern-pack03-via-dmopacktoolkit",
        import_method: "extract-pack03-and-read-itemdata-payloads",
        toolkit: "DmoPackToolkit",
        pack_files: vec![
            PackFileFingerprint {
                name: workspace
                    .pack03_hf
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Pack03.hf")
                    .to_string(),
                sha256: sha256_file(&workspace.pack03_hf)?,
            },
            PackFileFingerprint {
                name: workspace
                    .pack03_pf
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Pack03.pf")
                    .to_string(),
                sha256: sha256_file(&workspace.pack03_pf)?,
            },
        ],
        item_payload_path: item_payload
            .strip_prefix(&workspace.temp_root)
            .unwrap_or(&item_payload)
            .display()
            .to_string(),
        item_payload_sha256: sha256_file(&item_payload)?,
        cooltime_payload_path: cooltime_payload
            .strip_prefix(&workspace.temp_root)
            .unwrap_or(&cooltime_payload)
            .display()
            .to_string(),
        cooltime_payload_sha256: sha256_file(&cooltime_payload)?,
        item_name_source,
    };

    Ok((assets, manifest))
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
    evolution_assets: Option<&[EvolutionAsset]>,
    item_assets: Option<&[ItemAsset]>,
    digimon_assets: Option<&[DigimonAsset]>,
) -> Result<()> {
    let pool = PgPool::connect(database_url).await?;
    let mut tx = pool.begin().await?;

    if let Some(evolution_assets) = evolution_assets {
        sqlx::query("DELETE FROM evolution_assets")
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
    }

    if let Some(item_assets) = item_assets {
        sqlx::query("DELETE FROM item_assets")
            .execute(&mut *tx)
            .await?;

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
    }

    if let Some(digimon_assets) = digimon_assets {
        sqlx::query("DELETE FROM digimon_assets")
            .execute(&mut *tx)
            .await?;

        for asset in digimon_assets {
            sqlx::query(
                "INSERT INTO digimon_assets (digimon_id, payload) VALUES ($1, $2)
                 ON CONFLICT (digimon_id) DO UPDATE SET payload = EXCLUDED.payload",
            )
            .bind(asset.digimon_id)
            .bind(serde_json::to_value(asset)?)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = parse_args()?;

    let evolution_out = config.output_dir.join("evolution_assets.json");
    let item_out = config.output_dir.join("item_assets.json");
    let item_manifest_out = config.output_dir.join("item_assets.source.json");
    let digimon_out = config.output_dir.join("digimon_assets.json");
    let digimon_manifest_out = config.output_dir.join("digimon_assets.source.json");
    let decoded_pack03 = match &config.digimon_pack_source {
        Some(source) => Some(decode_pack03_workspace(source)?),
        None => None,
    };

    let evolution_assets = match &config.evolution_json {
        Some(path) => {
            let assets = parse_evolution_assets(path)?;
            write_json(&evolution_out, &assets)?;
            Some(assets)
        }
        None => None,
    };

    let item_assets = match &config.item_csv {
        Some(item_csv) => {
            let item_names = match &config.item_str_xml {
                Some(path) => parse_item_names(path)?,
                None => HashMap::new(),
            };
            let cooltimes = match &config.cooltime_xml {
                Some(path) => parse_cooltimes(path)?,
                None => HashMap::new(),
            };
            let assets = parse_item_assets(item_csv, &item_names, &cooltimes)?;
            write_json(&item_out, &assets)?;
            Some(assets)
        }
        None => match &decoded_pack03 {
            Some(workspace) => {
                let (item_names, item_name_source) = match &config.item_str_xml {
                    Some(path) => (parse_item_names(path)?, "noncanonical-item-str-xml"),
                    None => (
                        load_existing_item_names(&item_out)?,
                        "server-owned-catalog-carry-forward",
                    ),
                };
                let (assets, manifest) =
                    load_item_assets_from_decoded_pack03(workspace, &item_names, item_name_source)?;
                write_json(&item_out, &assets)?;
                write_json(&item_manifest_out, &manifest)?;
                Some(assets)
            }
            None => None,
        },
    };

    let digimon_assets = match &config.digimon_pack_source {
        Some(_) => {
            let workspace = decoded_pack03
                .as_ref()
                .context("decoded Pack03 workspace was not prepared")?;
            let (assets, manifest) = load_digimon_assets_from_decoded_pack03(workspace)?;
            write_json(&digimon_out, &assets)?;
            write_json(&digimon_manifest_out, &manifest)?;
            Some(assets)
        }
        None => None,
    };

    if let Some(database_url) = &config.database_url {
        sync_postgres(
            database_url,
            evolution_assets.as_deref(),
            item_assets.as_deref(),
            digimon_assets.as_deref(),
        )
        .await?;
    }

    let evolution_count = evolution_assets.as_ref().map_or(0, Vec::len);
    let item_count = item_assets.as_ref().map_or(0, Vec::len);
    let digimon_count = digimon_assets.as_ref().map_or(0, Vec::len);

    println!(
        "generated {} evolution assets, {} item assets, and {} digimon assets",
        evolution_count, item_count, digimon_count
    );
    if evolution_assets.is_some() {
        println!("wrote {}", evolution_out.display());
    }
    if item_assets.is_some() {
        println!("wrote {}", item_out.display());
        if item_manifest_out.exists() {
            println!("wrote {}", item_manifest_out.display());
        }
    }
    if digimon_assets.is_some() {
        println!("wrote {}", digimon_out.display());
        println!("wrote {}", digimon_manifest_out.display());
    }
    if config.database_url.is_some() {
        println!("postgres sync completed");
    }

    Ok(())
}
